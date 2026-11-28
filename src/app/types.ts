// Payloads of the commands and events in src-tauri/src/commands.rs and live.rs.

export interface WindowView {
  handle: number;
  title: string;
  processName: string;
  icon: string | null;
  /** A still of the window; null while it is minimized. */
  thumbnail: string | null;
  playingAudio: boolean;
}

/** A display that can be streamed whole. */
export interface MonitorView {
  handle: number;
  /** One-based, in the order Windows lists the displays. */
  number: number;
  /** The display model; empty when Windows reports none. */
  name: string;
  width: number;
  height: number;
  primary: boolean;
  thumbnail: string | null;
}

export type SourceKind = "window" | "monitor";

/** What the user picks to stream. */
export interface SourceRef {
  kind: SourceKind;
  handle: number;
}

export interface SourceView {
  kind: SourceKind;
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
  /** The apps a screen stream can carry; null for a window. */
  audioApps: AudioAppView[] | null;
}

/** An app with sound during a screen stream. */
export interface AudioAppView {
  processId: number;
  name: string;
  icon: string | null;
  included: boolean;
  /** From 0 to 1. */
  volume: number;
  /** Peak level since the previous snapshot, after the volume, from 0 to 1. */
  level: number;
}

/** "acrylic" when Windows paints the Acrylic material behind the window. */
export type Backdrop = "acrylic" | "solid";

export interface AppStatus {
  url: string;
  serverError: string | null;
  backdrop: Backdrop;
  live: LiveSnapshot | null;
}

export type EndReason = "windowClosed" | "captureFailed";

export type CommandErrorCode = "windowGone" | "videoUnavailable" | "audioUnavailable" | "internal";

export interface CommandError {
  code: CommandErrorCode;
  detail?: string;
}
