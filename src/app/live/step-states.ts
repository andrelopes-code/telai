import type { LiveSnapshot } from "../types";

export type StepState = "pending" | "attention" | "done";

export interface StepStates {
  open: StepState;
  mute: StepState;
  share: StepState;
}

/**
 * Progress of the steps that bring the stream into the call.
 *
 * The tab counts as muted only once the window was heard playing sound while
 * the browser stayed silent; before that, nothing proves it either way.
 * Sharing happens inside the call, which the app cannot observe.
 */
export function stepStates(snapshot: LiveSnapshot, soundHeard: boolean): StepStates {
  const open: StepState = snapshot.viewers > 0 ? "done" : "pending";

  let mute: StepState = "pending";
  if (open === "done" && snapshot.browserAudible) {
    mute = "attention";
  } else if (open === "done" && soundHeard) {
    mute = "done";
  }

  return { open, mute, share: "pending" };
}
