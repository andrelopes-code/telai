import { describe, expect, it } from "vitest";

import type { WindowView } from "../types";
import { groupWindows, selectedWindow } from "./group-windows";

describe("groupWindows", () => {
  it("puts windows playing sound first, keeping their order", () => {
    const windows = [
      window(1, "Notes", "notepad", false),
      window(2, "Demo.mp4 - Media Player", "player", true),
      window(3, "Browser", "browser", true),
    ];

    const groups = groupWindows(windows, "");

    expect(groups.playing.map((item) => item.handle)).toEqual([2, 3]);
    expect(groups.others.map((item) => item.handle)).toEqual([1]);
  });

  it("matches the title or the process name, ignoring case and accents", () => {
    const windows = [
      window(1, "Ação e Reação.mp4", "player", false),
      window(2, "Inbox", "Outlook", false),
    ];

    expect(groupWindows(windows, "acao").others.map((item) => item.handle)).toEqual([1]);
    expect(groupWindows(windows, " OUTLOOK ").others.map((item) => item.handle)).toEqual([2]);
    expect(groupWindows(windows, "nothing").others).toEqual([]);
  });
});

describe("selectedWindow", () => {
  const windows = [
    window(1, "Notes", "notepad", false),
    window(2, "Demo.mp4 - Media Player", "player", true),
  ];

  it("keeps the window the user picked", () => {
    expect(selectedWindow(windows, 1)).toBe(1);
  });

  it("falls back to the first window playing sound", () => {
    expect(selectedWindow(windows, null)).toBe(2);
    expect(selectedWindow(windows, 99)).toBe(2);
  });

  it("selects nothing when no window plays sound", () => {
    expect(selectedWindow([window(1, "Notes", "notepad", false)], null)).toBeNull();
  });
});

function window(
  handle: number,
  title: string,
  processName: string,
  playingAudio: boolean,
): WindowView {
  return { handle, title, processName, playingAudio, icon: null, thumbnail: null };
}
