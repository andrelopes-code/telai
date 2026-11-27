import type { WindowView } from "../types";

export interface WindowGroups {
  playing: WindowView[];
  others: WindowView[];
}

/**
 * Keeps the windows whose title or process name contains the query, ignoring
 * case and accents, and puts the ones playing sound first. Each group keeps
 * the front-to-back order of the input.
 */
export function groupWindows(windows: WindowView[], query: string): WindowGroups {
  const needle = normalize(query.trim());
  const matches = windows.filter(
    (window) =>
      normalize(window.title).includes(needle) || normalize(window.processName).includes(needle),
  );

  return {
    playing: matches.filter((window) => window.playingAudio),
    others: matches.filter((window) => !window.playingAudio),
  };
}

/**
 * The window the user picked while it is still open; before any pick, the
 * frontmost window that plays sound, which is the usual choice.
 */
export function selectedWindow(windows: WindowView[], picked: number | null): number | null {
  if (picked !== null && windows.some((window) => window.handle === picked)) {
    return picked;
  }

  return windows.find((window) => window.playingAudio)?.handle ?? null;
}

function normalize(value: string): string {
  return value
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();
}
