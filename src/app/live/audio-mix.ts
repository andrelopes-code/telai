import type { AudioAppView } from "../types";

/**
 * The apps in the order the panel lists them: the ones the stream carries
 * first, then the ones left out, each part by name. Processes of one app keep
 * the order they came in.
 */
export function mixRows(apps: AudioAppView[]): AudioAppView[] {
  return [...apps].sort((a, b) => {
    if (a.included !== b.included) {
      return a.included ? -1 : 1;
    }

    return a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
  });
}

/** The volume as a whole percentage, for the slider. */
export function volumePercent(volume: number): number {
  return Math.round(Math.max(0, Math.min(1, volume)) * 100);
}
