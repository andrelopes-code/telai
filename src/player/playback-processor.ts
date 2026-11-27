import { SAMPLE_RATE } from "../shared/protocol";
import { PROCESSOR_NAME } from "./processor-name";
import { SampleQueue } from "./sample-queue";

// 40 ms rides out WebSocket jitter on the same machine; 150 ms caps the
// delay between the picture and the sound.
const TARGET_FRAMES = (SAMPLE_RATE * 40) / 1000;
const MAX_FRAMES = (SAMPLE_RATE * 150) / 1000;

class PlaybackProcessor extends AudioWorkletProcessor {
  private readonly queue = new SampleQueue(TARGET_FRAMES, MAX_FRAMES);

  constructor() {
    super();

    this.port.onmessage = (event: MessageEvent<Float32Array>) => {
      this.queue.push(event.data);
    };
  }

  process(_inputs: Float32Array[][], outputs: Float32Array[][]): boolean {
    const [left, right] = outputs[0] ?? [];
    if (left && right) {
      this.queue.pull(left, right);
    }

    return true;
  }
}

registerProcessor(PROCESSOR_NAME, PlaybackProcessor);
