// Globals of the AudioWorklet scope, which the DOM library does not declare.

declare abstract class AudioWorkletProcessor {
  readonly port: MessagePort;

  abstract process(inputs: Float32Array[][], outputs: Float32Array[][]): boolean;
}

declare function registerProcessor(name: string, processor: new () => AudioWorkletProcessor): void;
