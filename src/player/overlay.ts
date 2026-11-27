import { strings } from "./strings";

export type OverlayMode = "offline" | "idle" | "audio" | "hidden";

const MESSAGES = {
  offline: { title: strings.offlineTitle, hint: strings.offlineHint },
  idle: { title: strings.idleTitle, hint: strings.idleHint },
  audio: { title: strings.audioTitle, hint: strings.audioHint },
} as const;

/** Only Chromium browsers include tab audio when a tab is shared. */
const sharesTabAudio = "userAgentData" in navigator;

/** The message over the video while there is nothing to watch or hear. */
export class Overlay {
  constructor(
    private readonly root: HTMLElement,
    private readonly title: HTMLElement,
    private readonly hint: HTMLElement,
  ) {}

  show(mode: OverlayMode): void {
    this.root.hidden = mode === "hidden";
    if (mode === "hidden") {
      return;
    }

    this.title.textContent = MESSAGES[mode].title;
    this.hint.textContent = sharesTabAudio ? MESSAGES[mode].hint : strings.browserHint;
  }
}
