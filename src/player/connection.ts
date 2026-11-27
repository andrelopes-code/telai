import { parsePacket, parseStatus } from "../shared/protocol";
import type { Status } from "../shared/protocol";

const RECONNECT_DELAY_MS = 1000;

export interface ConnectionHandlers {
  onOpen: () => void;
  onClose: () => void;
  onStatus: (status: Status) => void;
  onVideo: (jpeg: Uint8Array) => void;
  onAudio: (samples: Float32Array) => void;
}

/** Keeps a WebSocket to the app open, reconnecting whenever it drops. */
export function connect(url: string, handlers: ConnectionHandlers): void {
  const socket = new WebSocket(url);
  socket.binaryType = "arraybuffer";

  socket.addEventListener("open", handlers.onOpen);

  socket.addEventListener("message", (event: MessageEvent<string | ArrayBuffer>) => {
    dispatch(event.data, handlers);
  });

  socket.addEventListener("close", () => {
    handlers.onClose();
    setTimeout(() => {
      connect(url, handlers);
    }, RECONNECT_DELAY_MS);
  });
}

function dispatch(data: string | ArrayBuffer, handlers: ConnectionHandlers): void {
  if (typeof data === "string") {
    const status = parseStatus(data);
    if (status) {
      handlers.onStatus(status);
    }

    return;
  }

  const packet = parsePacket(data);
  if (packet?.kind === "video") {
    handlers.onVideo(packet.jpeg);
  }

  if (packet?.kind === "audio") {
    handlers.onAudio(packet.samples);
  }
}
