use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;

use wasapi::{AudioClient, Direction, SampleType, StreamMode, WasapiError, WaveFormat};

use crate::Error;

const SAMPLE_RATE: usize = 48_000;
const CHANNELS: usize = 2;
const BYTES_PER_SAMPLE: usize = 4;

/// How long the capture thread sleeps without audio before it checks for a
/// stop request. A silent process delivers no packets at all.
const WAIT_MS: u32 = 100;

/// Captures the audio rendered by one process and its children, as
/// interleaved stereo `f32` samples at 48 kHz.
pub struct AudioCapture {
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl AudioCapture {
    /// Starts capturing. Returns once the stream runs, or with the error that
    /// prevented it.
    pub fn start(
        process_id: u32,
        on_samples: impl FnMut(&[f32]) + Send + 'static,
    ) -> Result<Self, Error> {
        let stop = Arc::new(AtomicBool::new(false));
        let (started, start_result) = mpsc::channel();

        let thread = thread::Builder::new().name("audio-capture".into()).spawn({
            let stop = Arc::clone(&stop);
            move || capture(process_id, &stop, &started, on_samples)
        })?;

        match start_result.recv() {
            Ok(Ok(())) => Ok(Self {
                stop,
                thread: Some(thread),
            }),
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(Error::Audio(error))
            }
            Err(_) => {
                let _ = thread.join();
                Err(Error::AudioThreadExited)
            }
        }
    }

    pub fn stop(mut self) {
        self.stop_capture();
    }

    fn stop_capture(&mut self) {
        self.stop.store(true, Ordering::Relaxed);

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        self.stop_capture();
    }
}

fn capture(
    process_id: u32,
    stop: &AtomicBool,
    started: &mpsc::Sender<Result<(), WasapiError>>,
    mut on_samples: impl FnMut(&[f32]),
) {
    let stream = match open_stream(process_id) {
        Ok(stream) => {
            let _ = started.send(Ok(()));
            stream
        }
        Err(error) => {
            let _ = started.send(Err(error));
            return;
        }
    };

    if let Err(error) = read_until_stopped(&stream, stop, &mut on_samples) {
        tracing::error!(%error, "audio capture stopped");
    }

    let _ = stream.client.stop_stream();
}

struct Stream {
    client: AudioClient,
    capture: wasapi::AudioCaptureClient,
    event: wasapi::Handle,
}

fn open_stream(process_id: u32) -> Result<Stream, WasapiError> {
    wasapi::initialize_mta().ok()?;

    let format = WaveFormat::new(32, 32, SampleType::Float, SAMPLE_RATE, CHANNELS, None);
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: 0,
    };

    let mut client = AudioClient::new_application_loopback_client(process_id, true)?;
    client.initialize_client(&format, Direction::Capture, mode, None)?;

    let event = client.set_get_eventhandle()?;
    let capture = client.get_audiocaptureclient()?;
    client.start_stream()?;

    Ok(Stream {
        client,
        capture,
        event,
    })
}

fn read_until_stopped(
    stream: &Stream,
    stop: &AtomicBool,
    on_samples: &mut impl FnMut(&[f32]),
) -> Result<(), WasapiError> {
    let mut bytes = Vec::new();
    let mut samples = Vec::new();

    while !stop.load(Ordering::Relaxed) {
        match stream.event.wait_for_event(WAIT_MS) {
            Ok(()) | Err(WasapiError::EventTimeout) => {}
            Err(error) => return Err(error),
        }

        while let Some(frames) = stream.capture.get_next_packet_size()?
            && frames > 0
        {
            bytes.resize(frames as usize * CHANNELS * BYTES_PER_SAMPLE, 0);
            let (read, _) = stream.capture.read_from_device(&mut bytes)?;
            let len = read as usize * CHANNELS * BYTES_PER_SAMPLE;

            samples.clear();
            samples.extend(
                bytes[..len].chunks_exact(BYTES_PER_SAMPLE).map(|sample| {
                    f32::from_le_bytes(sample.try_into().expect("chunk of four bytes"))
                }),
            );
            on_samples(&samples);
        }
    }

    Ok(())
}
