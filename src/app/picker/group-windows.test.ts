import { describe, expect, it } from "vitest";

import type { MonitorView, WindowView } from "../types";
import { filterMonitors, groupWindows, selectedSource } from "./group-windows";

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

describe("selectedSource", () => {
  const windows = [
    window(1, "Notes", "notepad", false),
    window(2, "Demo.mp4 - Media Player", "player", true),
  ];
  const monitors = [monitor(10, 1, "DELL U2720Q")];

  it("keeps the window the user picked", () => {
    expect(selectedSource(windows, monitors, { kind: "window", handle: 1 })).toEqual({
      kind: "window",
      handle: 1,
    });
  });

  it("keeps a picked monitor while it is listed", () => {
    expect(selectedSource(windows, monitors, { kind: "monitor", handle: 10 })).toEqual({
      kind: "monitor",
      handle: 10,
    });
  });

  it("falls back to the first window playing sound", () => {
    const playing = { kind: "window", handle: 2 };

    expect(selectedSource(windows, monitors, null)).toEqual(playing);
    expect(selectedSource(windows, monitors, { kind: "window", handle: 99 })).toEqual(playing);
    expect(selectedSource(windows, monitors, { kind: "monitor", handle: 99 })).toEqual(playing);
  });

  it("does not take a window handle for a monitor", () => {
    expect(selectedSource(windows, monitors, { kind: "monitor", handle: 1 })).toEqual({
      kind: "window",
      handle: 2,
    });
  });

  it("selects nothing when no window plays sound", () => {
    expect(selectedSource([window(1, "Notes", "notepad", false)], monitors, null)).toBeNull();
  });
});

describe("filterMonitors", () => {
  it("matches the display name or its number, ignoring case", () => {
    const monitors = [monitor(10, 1, "DELL U2720Q"), monitor(11, 3, "")];

    expect(filterMonitors(monitors, "dell").map((item) => item.handle)).toEqual([10]);
    expect(filterMonitors(monitors, " 3 ").map((item) => item.handle)).toEqual([11]);
    expect(filterMonitors(monitors, "").map((item) => item.handle)).toEqual([10, 11]);
    expect(filterMonitors(monitors, "nothing")).toEqual([]);
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

function monitor(handle: number, number: number, name: string): MonitorView {
  return {
    handle,
    number,
    name,
    width: 1920,
    height: 1080,
    primary: number === 1,
    thumbnail: null,
  };
}
