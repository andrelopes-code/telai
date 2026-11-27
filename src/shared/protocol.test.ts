import { describe, expect, it } from "vitest";

import { parsePacket, parseStatus } from "./protocol";

describe("parsePacket", () => {
  it("reads a video packet as JPEG bytes", () => {
    const packet = parsePacket(bytes(1, 0, 0, 0, 0xff, 0xd8));

    expect(packet).toEqual({ kind: "video", jpeg: new Uint8Array([0xff, 0xd8]) });
  });

  it("reads an audio packet as float samples", () => {
    const data = new ArrayBuffer(12);
    new Uint8Array(data)[0] = 2;
    new DataView(data).setFloat32(4, 0.5, true);
    new DataView(data).setFloat32(8, -0.25, true);

    const packet = parsePacket(data);

    expect(packet).toEqual({ kind: "audio", samples: new Float32Array([0.5, -0.25]) });
  });

  it("rejects an audio payload that is not whole samples", () => {
    expect(parsePacket(bytes(2, 0, 0, 0, 1, 2))).toBeNull();
  });

  it("rejects unknown kinds and short packets", () => {
    expect(parsePacket(bytes(9, 0, 0, 0))).toBeNull();
    expect(parsePacket(bytes(1, 0))).toBeNull();
  });
});

describe("parseStatus", () => {
  it("reads a status message", () => {
    expect(parseStatus('{"live":true,"title":"Movie"}')).toEqual({ live: true, title: "Movie" });
  });

  it("rejects malformed messages", () => {
    expect(parseStatus("{")).toBeNull();
    expect(parseStatus('{"live":"yes","title":"Movie"}')).toBeNull();
    expect(parseStatus("null")).toBeNull();
  });
});

function bytes(...values: number[]): ArrayBuffer {
  return new Uint8Array(values).buffer;
}
