import { describe, expect, it } from "vitest";

import { text } from "../i18n/text";
import { endedMessage, errorMessage, pickerNotice } from "./error-message";

describe("errorMessage", () => {
  it("translates each command error code", () => {
    expect(errorMessage({ code: "windowGone" })).toBe(text.errorWindowGone);
    expect(errorMessage({ code: "videoUnavailable", detail: "x" })).toBe(
      text.errorVideoUnavailable,
    );
    expect(errorMessage({ code: "audioUnavailable", detail: "x" })).toBe(
      text.errorAudioUnavailable,
    );
  });

  it("falls back to a generic message for anything else", () => {
    expect(errorMessage(new Error("boom"))).toBe(text.errorInternal);
    expect(errorMessage("boom")).toBe(text.errorInternal);
  });
});

describe("endedMessage", () => {
  it("names why the stream ended", () => {
    expect(endedMessage("windowClosed")).toBe(text.endedWindowClosed);
    expect(endedMessage("captureFailed")).toBe(text.endedCaptureFailed);
  });
});

describe("pickerNotice", () => {
  it("prefers a start error over how the last stream ended", () => {
    expect(pickerNotice({ code: "windowGone" }, "windowClosed")).toBe(text.errorWindowGone);
    expect(pickerNotice(null, "windowClosed")).toBe(text.endedWindowClosed);
    expect(pickerNotice(null, null)).toBeNull();
  });
});
