import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";

import type { AppStatus, CommandError, EndReason, LiveSnapshot, WindowView } from "./types";

export const bridge = {
  status: () => invoke<AppStatus>("status"),
  listWindows: () => invoke<WindowView[]>("list_windows"),
  start: (handle: number) => invoke<LiveSnapshot>("start", { handle }),
  stop: () => invoke<undefined>("stop"),
  openPlayer: () => invoke<undefined>("open_player"),

  onLive: (listener: (snapshot: LiveSnapshot) => void): Promise<UnlistenFn> =>
    listen<LiveSnapshot>("live", (event) => {
      listener(event.payload);
    }),

  onEnded: (listener: (reason: EndReason) => void): Promise<UnlistenFn> =>
    listen<EndReason>("live-ended", (event) => {
      listener(event.payload);
    }),
};

export function isCommandError(error: unknown): error is CommandError {
  return typeof error === "object" && error !== null && "code" in error;
}
