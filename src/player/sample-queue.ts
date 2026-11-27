/**
 * Jitter buffer for interleaved stereo samples, read by the audio thread.
 *
 * Playback starts once `targetFrames` are buffered, and starts over after an
 * underrun. When more than `maxFrames` pile up, the oldest are dropped down
 * to the target, so latency never grows past the cap.
 */
export class SampleQueue {
  private readonly buffer: Float32Array;
  private readonly capacity: number;
  private readFrame = 0;
  private buffered = 0;
  private playing = false;

  constructor(
    private readonly targetFrames: number,
    private readonly maxFrames: number,
  ) {
    this.capacity = maxFrames * 2;
    this.buffer = new Float32Array(this.capacity * 2);
  }

  get bufferedFrames(): number {
    return this.buffered;
  }

  push(interleaved: Float32Array): void {
    const frames = Math.floor(interleaved.length / 2);
    if (frames > this.capacity) {
      return;
    }

    let writeFrame = (this.readFrame + this.buffered) % this.capacity;
    for (let frame = 0; frame < frames; frame++) {
      this.buffer[writeFrame * 2] = interleaved[frame * 2] ?? 0;
      this.buffer[writeFrame * 2 + 1] = interleaved[frame * 2 + 1] ?? 0;
      writeFrame = (writeFrame + 1) % this.capacity;
    }

    this.buffered += frames;
    if (this.buffered > this.maxFrames) {
      this.drop(this.buffered - this.targetFrames);
    }
  }

  /** Fills both channels, with silence for any frame not yet available. */
  pull(left: Float32Array, right: Float32Array): void {
    if (!this.playing && this.buffered >= this.targetFrames) {
      this.playing = true;
    }

    const available = this.playing ? Math.min(left.length, this.buffered) : 0;
    for (let frame = 0; frame < available; frame++) {
      const index = ((this.readFrame + frame) % this.capacity) * 2;
      left[frame] = this.buffer[index] ?? 0;
      right[frame] = this.buffer[index + 1] ?? 0;
    }

    left.fill(0, available);
    right.fill(0, available);
    this.drop(available);

    if (available < left.length) {
      this.playing = false;
    }
  }

  private drop(frames: number): void {
    this.readFrame = (this.readFrame + frames) % this.capacity;
    this.buffered -= frames;
  }
}
