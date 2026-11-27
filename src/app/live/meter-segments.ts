/** The meter spans -60 dBFS to 0 dBFS, like a broadcast peak meter. */
const FLOOR_DB = -60;

/** Number of lit segments for a peak level between 0 and 1. */
export function litSegments(level: number, segments: number): number {
  if (level <= 0) {
    return 0;
  }

  const decibels = 20 * Math.log10(Math.min(level, 1));
  const fraction = (decibels - FLOOR_DB) / -FLOOR_DB;

  return Math.max(0, Math.min(segments, Math.ceil(fraction * segments)));
}

export type SegmentTone = "low" | "high" | "peak";

/** Amber from -18 dBFS and red from -6 dBFS, as on broadcast peak meters. */
export function segmentTone(index: number, segments: number): SegmentTone {
  const decibels = FLOOR_DB + ((index + 1) / segments) * -FLOOR_DB;

  if (decibels > -6) {
    return "peak";
  }

  return decibels > -18 ? "high" : "low";
}
