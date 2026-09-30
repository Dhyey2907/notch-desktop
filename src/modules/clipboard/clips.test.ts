import { describe, expect, it } from "vitest";

import type { Clip } from "../../core/generated/Clip";
import { age, clipTitle, matchesQuery, taskTitle } from "./clips";

const clip = (overrides: Partial<Clip> = {}): Clip => ({
  id: "1",
  kind: "text",
  preview: "Hello   world\nsecond line",
  length: 25,
  files: [],
  at: "2026-09-30T12:00:00Z",
  pinned: false,
  app: "Chrome",
  ...overrides,
});

describe("clipboard list", () => {
  it("searches text, app and kind, every word", () => {
    expect(matchesQuery(clip(), "")).toBe(true);
    expect(matchesQuery(clip(), "WORLD")).toBe(true);
    expect(matchesQuery(clip(), "chrome hello")).toBe(true);
    expect(matchesQuery(clip(), "hello slack")).toBe(false);
    expect(matchesQuery(clip({ kind: "image", preview: "" }), "screenshot")).toBe(true);
  });

  it("describes each kind on one line", () => {
    expect(clipTitle(clip())).toBe("Hello world second line");
    expect(clipTitle(clip({ kind: "image", preview: "", width: 1920, height: 1080 }))).toBe(
      "Image · 1920×1080",
    );
    expect(clipTitle(clip({ kind: "files", preview: "a.pdf, b.png", files: ["C:/a.pdf", "C:/b.png"] }))).toBe(
      "2 files · a.pdf, b.png",
    );
  });

  it("offers a task only for short single-line text", () => {
    expect(taskTitle(clip({ preview: "Call the bank" }))).toBe("Call the bank");
    expect(taskTitle(clip())).toBeNull();
    expect(taskTitle(clip({ kind: "files", preview: "a.pdf" }))).toBeNull();
    expect(taskTitle(clip({ preview: "https://x.com", textKind: "url" }))).toBeNull();
  });

  it("shows how long ago", () => {
    const at = new Date("2026-09-30T12:00:00Z").getTime();
    expect(age("2026-09-30T12:00:00Z", at + 30_000)).toBe("now");
    expect(age("2026-09-30T12:00:00Z", at + 5 * 60_000)).toBe("5m");
    expect(age("2026-09-30T12:00:00Z", at + 3 * 3_600_000)).toBe("3h");
    expect(age("2026-09-30T12:00:00Z", at + 2 * 86_400_000)).toBe("2d");
  });
});
