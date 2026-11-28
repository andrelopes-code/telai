import { text } from "../i18n/text";
import type { MonitorView, SourceRef, WindowView } from "../types";

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

/** The monitors whose name or number contains the query, ignoring case and accents. */
export function filterMonitors(monitors: MonitorView[], query: string): MonitorView[] {
  const needle = normalize(query.trim());

  return monitors.filter((monitor) => normalize(monitorLabel(monitor)).includes(needle));
}

/** The display model, or "Screen 2" for a display that reports none. */
export function monitorLabel(monitor: MonitorView): string {
  const name = monitor.name.trim();

  return name === "" ? text.screenName(monitor.number) : name;
}

/**
 * The window or monitor the user picked while it is still there; before any
 * pick, the frontmost window that plays sound, which is the usual choice.
 */
export function selectedSource(
  windows: WindowView[],
  monitors: MonitorView[],
  picked: SourceRef | null,
): SourceRef | null {
  if (picked !== null) {
    const listed =
      picked.kind === "window"
        ? windows.some((window) => window.handle === picked.handle)
        : monitors.some((monitor) => monitor.handle === picked.handle);
    if (listed) {
      return picked;
    }
  }

  const playing = windows.find((window) => window.playingAudio);

  return playing === undefined ? null : { kind: "window", handle: playing.handle };
}

function normalize(value: string): string {
  return value
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();
}
