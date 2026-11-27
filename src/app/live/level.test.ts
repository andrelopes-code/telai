import { describe, expect, it } from "vitest";

import { levelPercent, levelZone } from "./level";

describe("levelPercent", () => {
  it("maps silence and full scale to the ends", () => {
    expect(levelPercent(0)).toBe(0);
    expect(levelPercent(1)).toBe(100);
    expect(levelPercent(1.5)).toBe(100);
  });

  it("is linear in decibels between -60 dBFS and 0 dBFS", () => {
    // 0.1 is -20 dBFS: 40 of 60 decibels above the floor.
    expect(levelPercent(0.1)).toBeCloseTo(66.67, 2);
    // 0.001 is -60 dBFS, the floor.
    expect(levelPercent(0.001)).toBeCloseTo(0, 6);
    expect(levelPercent(0.0001)).toBe(0);
  });
});

describe("levelZone", () => {
  it("turns amber above 70% and red above 90%", () => {
    expect(levelZone(70)).toBe("normal");
    expect(levelZone(71)).toBe("high");
    expect(levelZone(90)).toBe("high");
    expect(levelZone(91)).toBe("peak");
  });
});
