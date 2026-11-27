import { CHANNELS, SAMPLE_RATE } from "../shared/protocol";
import processorUrl from "./playback-processor.ts?worker&url";
import { PROCESSOR_NAME } from "./processor-name";

/** Plays the captured audio. Browsers keep it suspended until a click. */
export class AudioOutput {
  private readonly context = new AudioContext({
    sampleRate: SAMPLE_RATE,
    latencyHint: "interactive",
  });
  private readonly node: Promise<AudioWorkletNode>;

  constructor() {
    this.node = this.createNode();
  }

  get suspended(): boolean {
    return this.context.state !== "running";
  }

  onStateChange(listener: () => void): void {
    this.context.addEventListener("statechange", listener);
  }

  resume(): void {
    void this.context.resume();
  }

  play(samples: Float32Array): void {
    void this.node.then((node) => {
      node.port.postMessage(samples, [samples.buffer]);
    });
  }

  private async createNode(): Promise<AudioWorkletNode> {
    await this.context.audioWorklet.addModule(processorUrl);

    const node = new AudioWorkletNode(this.context, PROCESSOR_NAME, {
      numberOfInputs: 0,
      outputChannelCount: [CHANNELS],
    });
    node.connect(this.context.destination);

    return node;
  }
}
