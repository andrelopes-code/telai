import { isCommandError } from "../bridge";
import { text } from "../i18n/text";
import type { EndReason } from "../types";

/** The sentence shown for a failed command. */
export function errorMessage(error: unknown): string {
  if (!isCommandError(error)) {
    return text.errorInternal;
  }

  switch (error.code) {
    case "windowGone":
      return text.errorWindowGone;
    case "videoUnavailable":
      return text.errorVideoUnavailable;
    case "audioUnavailable":
      return text.errorAudioUnavailable;
    case "internal":
      return text.errorInternal;
  }
}

/** The sentence shown after a stream ended on its own. */
export function endedMessage(reason: EndReason): string {
  return reason === "windowClosed" ? text.endedWindowClosed : text.endedCaptureFailed;
}

/** What the picker reports about the last attempt, if anything. */
export function pickerNotice(error: unknown, ended: EndReason | null): string | null {
  if (error !== null) {
    return errorMessage(error);
  }

  return ended === null ? null : endedMessage(ended);
}
