import "./player.css";

import type { Status } from "../shared/protocol";
import { AudioOutput } from "./audio-output";
import { connect } from "./connection";
import { Overlay } from "./overlay";
import type { OverlayMode } from "./overlay";
import { strings } from "./strings";
import { VideoRenderer } from "./video-renderer";

const renderer = new VideoRenderer(element("screen") as HTMLCanvasElement);
const overlay = new Overlay(element("overlay"), element("overlay-title"), element("overlay-hint"));
const audio = new AudioOutput();

let connected = false;
let status: Status = { live: false, title: "" };

function overlayMode(): OverlayMode {
  if (!connected) {
    return "offline";
  }

  if (!status.live) {
    return "idle";
  }

  return audio.suspended ? "audio" : "hidden";
}

function refresh(): void {
  overlay.show(overlayMode());
  document.title = status.live ? strings.documentTitle(status.title) : "Telai";
}

connect(`ws://${location.host}/ws`, {
  onOpen: () => {
    connected = true;
    refresh();
  },
  onClose: () => {
    connected = false;
    renderer.clear();
    refresh();
  },
  onStatus: (next) => {
    status = next;
    if (!next.live) {
      renderer.clear();
    }

    refresh();
  },
  onVideo: (jpeg) => {
    renderer.draw(jpeg);
  },
  onAudio: (samples) => {
    audio.play(samples);
  },
});

audio.onStateChange(refresh);
document.addEventListener("pointerdown", () => {
  audio.resume();
});
refresh();

function element(id: string): HTMLElement {
  const found = document.getElementById(id);
  if (!found) {
    throw new Error(`#${id} is missing from player.html`);
  }

  return found;
}
