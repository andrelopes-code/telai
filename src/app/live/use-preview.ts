import { useEffect } from "react";
import type { RefObject } from "react";

import { VideoRenderer } from "../../player/video-renderer";
import { parsePacket } from "../../shared/protocol";

const RECONNECT_DELAY_MS = 1000;

/** Draws the video-only preview stream of the player server on a canvas. */
export function usePreview(url: string, canvas: RefObject<HTMLCanvasElement | null>): void {
  useEffect(() => {
    if (!canvas.current) {
      return;
    }

    const renderer = new VideoRenderer(canvas.current);
    const previewUrl = `${url.replace(/^http/, "ws")}/preview`;
    let socket: WebSocket | null = null;
    let retry: ReturnType<typeof setTimeout> | undefined;
    let active = true;

    function connect(): void {
      socket = new WebSocket(previewUrl);
      socket.binaryType = "arraybuffer";

      socket.addEventListener("message", (event: MessageEvent<ArrayBuffer>) => {
        const packet = parsePacket(event.data);
        if (packet?.kind === "video") {
          renderer.draw(packet.jpeg);
        }
      });

      socket.addEventListener("close", () => {
        if (active) {
          retry = setTimeout(connect, RECONNECT_DELAY_MS);
        }
      });
    }

    connect();

    return () => {
      active = false;
      clearTimeout(retry);
      socket?.close();
    };
  }, [url, canvas]);
}
