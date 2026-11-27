import { describe, expect, it } from "vitest";

import { formatElapsed } from "./format-elapsed";

describe("formatElapsed", () => {
  it("shows minutes and seconds under an hour", () => {
    expect(formatElapsed(0)).toBe("0:00");
    expect(formatElapsed(65_999)).toBe("1:05");
    expect(formatElapsed(3_599_000)).toBe("59:59");
  });

  it("adds hours from one hour on", () => {
    expect(formatElapsed(3_600_000)).toBe("1:00:00");
    expect(formatElapsed(7_384_000)).toBe("2:03:04");
  });

  it("treats a negative duration as zero", () => {
    expect(formatElapsed(-500)).toBe("0:00");
  });
});
