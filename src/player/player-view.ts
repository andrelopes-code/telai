import { strings } from "./strings";

export type PlayerMode = "offline" | "idle" | "audio" | "live";

const CARDS = {
  offline: { title: strings.offlineTitle, hint: strings.offlineHint },
  idle: { title: strings.idleTitle, hint: strings.idleHint },
  browser: { title: strings.browserTitle, hint: strings.browserHint },
} as const;

/** Only Chromium browsers include tab audio when a tab is shared. */
const sharesTabAudio = "userAgentData" in navigator;

/** The parts of the page around the video: the waiting card and the sound button. */
export class PlayerView {
  constructor(
    private readonly root: HTMLElement,
    private readonly card: HTMLElement,
    private readonly cardTitle: HTMLElement,
    private readonly cardHint: HTMLElement,
    private readonly unlock: HTMLElement,
  ) {}

  show(mode: PlayerMode): void {
    const waiting = mode === "offline" || mode === "idle";

    this.root.toggleAttribute("data-idle", waiting);
    this.card.hidden = !waiting;
    this.unlock.hidden = mode !== "audio";

    if (waiting) {
      const card = sharesTabAudio ? CARDS[mode] : CARDS.browser;
      this.cardTitle.textContent = card.title;
      this.cardHint.textContent = card.hint;
    }
  }
}
