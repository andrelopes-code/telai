import "@fontsource-variable/geist";
import "../styles/base.css";
import "./player.css";

import type { Status } from "../shared/protocol";
import { AudioOutput } from "./audio-output";
import { connect } from "./connection";
import { PlayerView } from "./player-view";
import type { PlayerMode } from "./player-view";
import { strings } from "./strings";
import { VideoRenderer } from "./video-renderer";

const renderer = new VideoRenderer(element("screen") as HTMLCanvasElement);
const view = new PlayerView(
  element("player"),
  element("card"),
  element("card-title"),
  element("card-hint"),
  element("unlock"),
);
const audio = new AudioOutput();

let connected = false;
let status: Status = { live: false, title: "" };

function mode(): PlayerMode {
  if (!connected) {
    return "offline";
  }

  if (!status.live) {
    return "idle";
  }

  return audio.suspended ? "audio" : "live";
}

function refresh(): void {
  view.show(mode());
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

element("unlock-label").textContent = strings.unlock;
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
