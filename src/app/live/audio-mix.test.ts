import { describe, expect, it } from "vitest";

import type { AudioAppView } from "../types";
import { mixRows } from "./audio-mix";

describe("mixRows", () => {
  it("lists the included apps first, each part by name ignoring case", () => {
    const apps = [
      app(1, "vlc", true),
      app(2, "Discord", false),
      app(3, "brave", true),
      app(4, "Audacity", false),
    ];

    expect(mixRows(apps).map((row) => row.processId)).toEqual([3, 1, 4, 2]);
  });

  it("keeps the order of several processes of one app", () => {
    const apps = [app(9, "vlc", true), app(5, "vlc", true)];

    expect(mixRows(apps).map((row) => row.processId)).toEqual([9, 5]);
  });

  it("does not change the list it was given", () => {
    const apps = [app(1, "vlc", true), app(2, "brave", true)];

    mixRows(apps);

    expect(apps.map((row) => row.processId)).toEqual([1, 2]);
  });
});

function app(processId: number, name: string, included: boolean): AudioAppView {
  return { processId, name, icon: null, included, volume: 1, level: 0 };
}
