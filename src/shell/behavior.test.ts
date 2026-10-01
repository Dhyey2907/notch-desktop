import { describe, expect, it } from "vitest";

import { hideOverFullscreen, inQuietHours, minutesToTime, timeToMinutes } from "./behavior";

const at = (h: number, m = 0) => new Date(2026, 9, 1, h, m);

describe("quiet hours", () => {
  it("handles a window that runs past midnight", () => {
    const quiet = { enabled: true, startMin: 22 * 60, endMin: 7 * 60 };
    expect(inQuietHours(quiet, at(23))).toBe(true);
    expect(inQuietHours(quiet, at(3))).toBe(true);
    expect(inQuietHours(quiet, at(7))).toBe(false);
    expect(inQuietHours(quiet, at(12))).toBe(false);
  });

  it("handles a daytime window and being off", () => {
    const quiet = { enabled: true, startMin: 13 * 60, endMin: 14 * 60 + 30 };
    expect(inQuietHours(quiet, at(13, 10))).toBe(true);
    expect(inQuietHours(quiet, at(14, 30))).toBe(false);
    expect(inQuietHours({ ...quiet, enabled: false }, at(13, 10))).toBe(false);
  });

  it("converts times", () => {
    expect(minutesToTime(22 * 60 + 5)).toBe("22:05");
    expect(timeToMinutes("7:30")).toBe(450);
    expect(timeToMinutes("24:00")).toBeNull();
    expect(timeToMinutes("nope")).toBeNull();
  });
});

describe("fullscreen", () => {
  it("follows the chosen behavior", () => {
    const card = { priority: "normal" };
    const approval = { priority: "critical" };
    expect(hideOverFullscreen("alerts", true, null)).toBe(true);
    expect(hideOverFullscreen("alerts", true, card)).toBe(false);
    expect(hideOverFullscreen("hide", true, card)).toBe(true);
    expect(hideOverFullscreen("hide", true, approval)).toBe(false);
    expect(hideOverFullscreen("show", true, null)).toBe(false);
    expect(hideOverFullscreen("alerts", false, null)).toBe(false);
  });
});
