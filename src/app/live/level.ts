/** Lowest level the line shows: -60 dBFS. */
const FLOOR_DB = -60;

export type LevelZone = "normal" | "high" | "peak";

/** Width of the level line, from 0 to 100, linear in decibels. */
export function levelPercent(level: number): number {
  if (level <= 0) {
    return 0;
  }

  const decibels = 20 * Math.log10(Math.min(level, 1));

  return Math.max(0, Math.min(100, ((decibels - FLOOR_DB) / -FLOOR_DB) * 100));
}

/** Amber from -18 dBFS (70%) and red from -6 dBFS (90%), as on broadcast meters. */
export function levelZone(percent: number): LevelZone {
  if (percent > 90) {
    return "peak";
  }

  return percent > 70 ? "high" : "normal";
}
