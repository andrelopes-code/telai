// Wire format of the player WebSocket. Mirrors crates/stream/src/protocol.rs.

export const HEADER_LENGTH = 4;
export const SAMPLE_RATE = 48_000;
export const CHANNELS = 2;

const VIDEO = 1;
const AUDIO = 2;

export interface Status {
  live: boolean;
  title: string;
}

export type Packet = { kind: "video"; jpeg: Uint8Array } | { kind: "audio"; samples: Float32Array };

export function parsePacket(data: ArrayBuffer): Packet | null {
  if (data.byteLength < HEADER_LENGTH) {
    return null;
  }

  const kind = new Uint8Array(data, 0, 1)[0];
  if (kind === VIDEO) {
    return { kind: "video", jpeg: new Uint8Array(data, HEADER_LENGTH) };
  }

  const payloadLength = data.byteLength - HEADER_LENGTH;
  if (kind === AUDIO && payloadLength % Float32Array.BYTES_PER_ELEMENT === 0) {
    return { kind: "audio", samples: new Float32Array(data, HEADER_LENGTH) };
  }

  return null;
}

export function parseStatus(text: string): Status | null {
  try {
    const value: unknown = JSON.parse(text);

    return isStatus(value) ? value : null;
  } catch {
    return null;
  }
}

function isStatus(value: unknown): value is Status {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const record = value as Record<string, unknown>;

  return typeof record.live === "boolean" && typeof record.title === "string";
}
