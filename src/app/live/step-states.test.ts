import { describe, expect, it } from "vitest";

import { liveStatus, liveSteps } from "./step-states";

describe("liveSteps", () => {
  it("starts at opening the player", () => {
    expect(liveSteps({ viewers: 0, browserAudible: true, soundHeard: true })).toEqual([
      "current",
      "pending",
      "pending",
    ]);
  });

  it("asks for attention while the browser plays the sound", () => {
    expect(liveSteps({ viewers: 1, browserAudible: true, soundHeard: true })).toEqual([
      "done",
      "attention",
      "pending",
    ]);
  });

  it("waits for sound before confirming the mute", () => {
    expect(liveSteps({ viewers: 1, browserAudible: false, soundHeard: false })).toEqual([
      "done",
      "current",
      "pending",
    ]);
  });

  it("leaves sharing as the current step once the tab is muted", () => {
    expect(liveSteps({ viewers: 2, browserAudible: false, soundHeard: true })).toEqual([
      "done",
      "done",
      "current",
    ]);
  });
});

describe("liveStatus", () => {
  it("names the first unfinished step", () => {
    expect(liveStatus(["current", "pending", "pending"])).toBe("open");
    expect(liveStatus(["done", "current", "pending"])).toBe("mute");
    expect(liveStatus(["done", "attention", "pending"])).toBe("muteAttention");
    expect(liveStatus(["done", "done", "current"])).toBe("ready");
  });
});
