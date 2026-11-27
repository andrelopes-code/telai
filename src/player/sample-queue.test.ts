import { describe, expect, it } from "vitest";

import { SampleQueue } from "./sample-queue";

describe("SampleQueue", () => {
  it("stays silent until the target is buffered", () => {
    const queue = new SampleQueue(4, 16);
    queue.push(stereo([1, 2, 3]));

    const [left, right] = pull(queue, 2);

    expect(left).toEqual(new Float32Array([0, 0]));
    expect(right).toEqual(new Float32Array([0, 0]));
    expect(queue.bufferedFrames).toBe(3);
  });

  it("plays frames in order once the target is reached", () => {
    const queue = new SampleQueue(2, 16);
    queue.push(stereo([1, 2, 3]));

    const [left, right] = pull(queue, 3);

    expect(left).toEqual(new Float32Array([1, 2, 3]));
    expect(right).toEqual(new Float32Array([-1, -2, -3]));
  });

  it("pads an underrun with silence and buffers again", () => {
    const queue = new SampleQueue(2, 16);
    queue.push(stereo([1, 2]));

    const [first] = pull(queue, 3);
    queue.push(stereo([5]));
    const [second] = pull(queue, 1);

    expect(first).toEqual(new Float32Array([1, 2, 0]));
    expect(second).toEqual(new Float32Array([0]));
  });

  it("drops the oldest frames down to the target past the cap", () => {
    const queue = new SampleQueue(2, 4);
    queue.push(stereo([1, 2, 3, 4, 5]));

    const [left] = pull(queue, 2);

    expect(left).toEqual(new Float32Array([4, 5]));
  });

  it("wraps around the ring buffer", () => {
    const queue = new SampleQueue(1, 3);

    for (let round = 0; round < 5; round++) {
      queue.push(stereo([round * 2, round * 2 + 1]));
      const [left] = pull(queue, 2);

      expect(left).toEqual(new Float32Array([round * 2, round * 2 + 1]));
    }
  });
});

/** Left channel holds the values, right channel their negation. */
function stereo(values: number[]): Float32Array {
  return new Float32Array(values.flatMap((value) => [value, -value]));
}

function pull(queue: SampleQueue, frames: number): [Float32Array, Float32Array] {
  const left = new Float32Array(frames);
  const right = new Float32Array(frames);
  queue.pull(left, right);

  return [left, right];
}
