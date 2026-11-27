import { describe, expect, it } from "vitest";

import type { LiveSnapshot } from "../types";
import { stepStates } from "./step-states";

describe("stepStates", () => {
  it("leaves every step pending until the player opens", () => {
    expect(stepStates(snapshot({ viewers: 0 }), true)).toEqual({
      open: "pending",
      mute: "pending",
      share: "pending",
    });
  });

  it("asks to mute the tab while the browser plays the sound", () => {
    expect(stepStates(snapshot({ viewers: 1, browserAudible: true }), true).mute).toBe("attention");
  });

  it("confirms the mute once sound played and the browser stayed silent", () => {
    expect(stepStates(snapshot({ viewers: 1 }), true)).toEqual({
      open: "done",
      mute: "done",
      share: "pending",
    });
  });

  it("keeps the mute pending while the window is silent", () => {
    expect(stepStates(snapshot({ viewers: 2 }), false).mute).toBe("pending");
  });
});

function snapshot(overrides: Partial<LiveSnapshot>): LiveSnapshot {
  return {
    source: { handle: 1, title: "Movie", processName: "vlc", icon: null },
    startedAt: 0,
    viewers: 0,
    browserAudible: false,
    minimized: false,
    level: 0,
    ...overrides,
  };
}
