// Payloads of the commands and events in src-tauri/src/commands.rs and live.rs.

export interface WindowView {
  handle: number;
  title: string;
  processName: string;
  icon: string | null;
  playingAudio: boolean;
}

export interface SourceView {
  handle: number;
  title: string;
  processName: string;
  icon: string | null;
}

export interface LiveSnapshot {
  source: SourceView;
  /** Unix time in milliseconds. */
  startedAt: number;
  viewers: number;
  /** The player tab plays the audio aloud on this PC. */
  browserAudible: boolean;
  minimized: boolean;
  /** Peak audio level since the previous snapshot, from 0 to 1. */
  level: number;
}

export interface AppStatus {
  url: string;
  serverError: string | null;
  live: LiveSnapshot | null;
}

export type EndReason = "windowClosed" | "captureFailed";

export type CommandErrorCode = "windowGone" | "videoUnavailable" | "audioUnavailable" | "internal";

export interface CommandError {
  code: CommandErrorCode;
  detail?: string;
}
