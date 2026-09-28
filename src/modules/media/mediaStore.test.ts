import { describe, expect, it } from "vitest";

import type { MediaState } from "../../core/generated/MediaState";
import { currentPositionMs, formatTime, shouldAnnounce } from "./mediaStore";

const base: MediaState = {
  sourceApp: "Spotify",
  title: "Song",
  artist: "Artist",
  album: "Album",
  status: "playing",
  positionMs: 10_000,
  durationMs: 60_000,
  positionUpdatedAt: "2026-09-28T00:00:00.000Z",
  rate: 1,
  capabilities: { playPause: true, next: true, previous: true, seek: true },
  mock: false,
};
const t0 = Date.parse(base.positionUpdatedAt);

describe("shouldAnnounce", () => {
  it("ignores the first reading at startup", () => {
    expect(shouldAnnounce(undefined, base)).toBe(false);
  });

  it("announces a different track that is playing", () => {
    expect(shouldAnnounce(base, { ...base, title: "Next song" })).toBe(true);
    expect(shouldAnnounce(null, base)).toBe(true);
  });

  it("stays quiet for pause, seek, or a paused new track", () => {
    expect(shouldAnnounce(base, { ...base, status: "paused" })).toBe(false);
    expect(shouldAnnounce(base, { ...base, positionMs: 30_000 })).toBe(false);
    expect(shouldAnnounce(base, { ...base, title: "Other", status: "paused" })).toBe(false);
  });
});

describe("currentPositionMs", () => {
  it("advances while playing and respects playback rate", () => {
    expect(currentPositionMs(base, t0 + 5_000)).toBe(15_000);
    expect(currentPositionMs({ ...base, rate: 2 }, t0 + 5_000)).toBe(20_000);
  });

  it("holds still when paused and never passes the end", () => {
    expect(currentPositionMs({ ...base, status: "paused" }, t0 + 5_000)).toBe(10_000);
    expect(currentPositionMs(base, t0 + 999_000)).toBe(60_000);
  });

  it("is unknown when the source reports no timeline", () => {
    expect(currentPositionMs({ ...base, positionMs: undefined }, t0)).toBeNull();
  });
});

it("formats times", () => {
  expect(formatTime(0)).toBe("0:00");
  expect(formatTime(75_000)).toBe("1:15");
  expect(formatTime(3_725_000)).toBe("1:02:05");
});
