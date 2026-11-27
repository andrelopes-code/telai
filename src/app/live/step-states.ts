export type StepState = "pending" | "current" | "attention" | "done";

export type LiveStatusKind = "open" | "mute" | "muteAttention" | "ready";

export interface StepInputs {
  viewers: number;
  browserAudible: boolean;
  soundHeard: boolean;
}

/**
 * Progress of the three steps that bring the stream into the call: open the
 * player, mute its tab, share it.
 *
 * The tab counts as muted once the window played sound while the browser
 * stayed silent. Sharing happens inside the call, which the app cannot
 * observe, so it becomes the current step and never completes.
 */
export function liveSteps({ viewers, browserAudible, soundHeard }: StepInputs): StepState[] {
  if (viewers === 0) {
    return ["current", "pending", "pending"];
  }

  if (browserAudible) {
    return ["done", "attention", "pending"];
  }

  if (!soundHeard) {
    return ["done", "current", "pending"];
  }

  return ["done", "done", "current"];
}

/** The sentence the live screen leads with, by the first unfinished step. */
export function liveStatus(steps: StepState[]): LiveStatusKind {
  const [open, mute] = steps;
  if (open !== "done") {
    return "open";
  }

  if (mute === "attention") {
    return "muteAttention";
  }

  return mute === "done" ? "ready" : "mute";
}
