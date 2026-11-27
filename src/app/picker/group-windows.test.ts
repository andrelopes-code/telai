import { describe, expect, it } from "vitest";

import type { WindowView } from "../types";
import { groupWindows } from "./group-windows";

describe("groupWindows", () => {
  it("puts windows playing sound first, keeping their order", () => {
    const windows = [
      window(1, "Notes", "notepad", false),
      window(2, "Movie.mkv - VLC media player", "vlc", true),
      window(3, "Stremio", "stremio", true),
    ];

    const groups = groupWindows(windows, "");

    expect(groups.playing.map((item) => item.handle)).toEqual([2, 3]);
    expect(groups.others.map((item) => item.handle)).toEqual([1]);
  });

  it("matches the title or the process name, ignoring case and accents", () => {
    const windows = [
      window(1, "Ação e Reação.mp4", "vlc", false),
      window(2, "Inbox", "Outlook", false),
    ];

    expect(groupWindows(windows, "acao").others.map((item) => item.handle)).toEqual([1]);
    expect(groupWindows(windows, " OUTLOOK ").others.map((item) => item.handle)).toEqual([2]);
    expect(groupWindows(windows, "nothing").others).toEqual([]);
  });
});

function window(
  handle: number,
  title: string,
  processName: string,
  playingAudio: boolean,
): WindowView {
  return { handle, title, processName, playingAudio, icon: null };
}
