import { describe, expect, it } from "vitest";

import { litSegments, segmentTone } from "./meter-segments";

describe("litSegments", () => {
  it("lights nothing for silence and everything at full scale", () => {
    expect(litSegments(0, 20)).toBe(0);
    expect(litSegments(1, 20)).toBe(20);
    expect(litSegments(1.5, 20)).toBe(20);
  });

  it("maps the level on a decibel scale", () => {
    expect(litSegments(0.1, 20)).toBe(14);
    expect(litSegments(0.001, 20)).toBe(0);
    expect(litSegments(0.5, 20)).toBe(18);
  });
});

describe("segmentTone", () => {
  it("colors the top of the scale amber, then red", () => {
    expect(segmentTone(0, 20)).toBe("low");
    expect(segmentTone(13, 20)).toBe("low");
    expect(segmentTone(14, 20)).toBe("high");
    expect(segmentTone(17, 20)).toBe("high");
    expect(segmentTone(18, 20)).toBe("peak");
    expect(segmentTone(19, 20)).toBe("peak");
  });
});
