use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use crate::encoder::{Frame, FrameEncoder};
use crate::hub::Hub;

/// A 1080p frame at quality 95 and 4:4:4 takes up to 100 ms on one core, so
/// 30 fps needs several threads. Half the cores leaves the rest to the call.
const MIN_WORKERS: usize = 2;
const MAX_WORKERS: usize = 6;

struct Job {
    sequence: u64,
    width: u32,
    height: u32,
    bgra: Vec<u8>,
}

struct Shared {
    hub: Hub,
    jobs: Mutex<Receiver<Job>>,
    free_buffers: Mutex<Vec<Vec<u8>>>,
    last_published: Mutex<u64>,
}

/// Encodes frames on worker threads and publishes them to the [`Hub`].
///
/// A frame submitted while every worker is busy is dropped, so encoding never
/// queues up latency. A frame that finishes after a newer one is discarded.
pub struct EncoderPool {
    shared: Arc<Shared>,
    jobs: Option<SyncSender<Job>>,
    workers: Vec<thread::JoinHandle<()>>,
    next_sequence: u64,
}

impl EncoderPool {
    /// # Panics
    ///
    /// Panics when the operating system refuses to start a thread.
    #[must_use]
    pub fn new(hub: Hub) -> Self {
        // A zero-capacity channel accepts a job only when a worker waits for one.
        let (jobs, receiver) = mpsc::sync_channel(0);
        let shared = Arc::new(Shared {
            hub,
            jobs: Mutex::new(receiver),
            free_buffers: Mutex::new(Vec::new()),
            last_published: Mutex::new(0),
        });

        let count = thread::available_parallelism()
            .map_or(MIN_WORKERS, |cores| cores.get() / 2)
            .clamp(MIN_WORKERS, MAX_WORKERS);
        let workers = (0..count)
            .map(|index| {
                let shared = Arc::clone(&shared);
                thread::Builder::new()
                    .name(format!("frame-encoder-{index}"))
                    .spawn(move || run_worker(&shared))
                    .expect("spawn encoder thread")
            })
            .collect();

        Self {
            shared,
            jobs: Some(jobs),
            workers,
            next_sequence: 1,
        }
    }

    /// Returns an empty buffer for the next frame, reusing a released one.
    #[must_use]
    pub fn buffer(&self) -> Vec<u8> {
        let mut free = lock(&self.shared.free_buffers);
        let mut buffer = free.pop().unwrap_or_default();
        buffer.clear();

        buffer
    }

    /// Queues a frame whose pixels are tightly packed BGRA rows. Returns
    /// `false` when every worker is busy and the frame was dropped.
    pub fn submit(&mut self, width: u32, height: u32, bgra: Vec<u8>) -> bool {
        let Some(jobs) = &self.jobs else {
            return false;
        };

        let job = Job {
            sequence: self.next_sequence,
            width,
            height,
            bgra,
        };
        self.next_sequence += 1;

        match jobs.try_send(job) {
            Ok(()) => true,
            Err(TrySendError::Full(job) | TrySendError::Disconnected(job)) => {
                release(&self.shared, job.bgra);
                false
            }
        }
    }
}

impl Drop for EncoderPool {
    fn drop(&mut self) {
        self.jobs = None;

        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

fn run_worker(shared: &Shared) {
    let mut encoder = FrameEncoder::new();

    loop {
        let job = lock(&shared.jobs).recv();
        let Ok(job) = job else {
            return;
        };

        let frame = Frame {
            width: job.width,
            height: job.height,
            bgra: &job.bgra,
        };
        match encoder.encode(frame) {
            Ok(packet) => {
                let mut last_published = lock(&shared.last_published);
                if job.sequence > *last_published {
                    *last_published = job.sequence;
                    shared.hub.publish_video(packet);
                }
            }
            Err(error) => tracing::warn!(%error, "dropping a frame that failed to encode"),
        }

        release(shared, job.bgra);
    }
}

fn release(shared: &Shared, buffer: Vec<u8>) {
    lock(&shared.free_buffers).push(buffer);
}

/// A worker that panicked mid-frame leaves no broken invariant behind.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn submitted_frames_are_published_as_video_packets() {
        let hub = Hub::new();
        let mut pool = EncoderPool::new(hub.clone());
        let mut bgra = pool.buffer();
        bgra.resize(16 * 16 * 4, 128);

        let accepted = submit_until_accepted(&mut pool, &bgra);
        drop(pool);

        assert!(accepted);
        let frame = hub.latest_video().expect("a published frame");
        assert_eq!(frame[0], crate::protocol::PacketKind::Video as u8);
    }

    #[test]
    fn released_buffers_are_reused() {
        let hub = Hub::new();
        let mut pool = EncoderPool::new(hub);
        let mut bgra = pool.buffer();
        bgra.resize(16 * 16 * 4, 0);
        let capacity = bgra.capacity();
        submit_until_accepted(&mut pool, &bgra);

        let reused = wait_for_buffer(&pool, capacity);

        assert!(reused);
    }

    fn submit_until_accepted(pool: &mut EncoderPool, bgra: &[u8]) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if pool.submit(16, 16, bgra.to_vec()) {
                return true;
            }
            thread::sleep(Duration::from_millis(5));
        }

        false
    }

    fn wait_for_buffer(pool: &EncoderPool, capacity: usize) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let buffer = pool.buffer();
            if buffer.capacity() >= capacity {
                return true;
            }
            thread::sleep(Duration::from_millis(5));
        }

        false
    }
}
