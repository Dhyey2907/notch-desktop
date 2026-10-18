import { describe, expect, it } from "vitest";

import { lockLines } from "./lockLines";

const now = Date.parse("2026-10-18T09:00:00");

describe("lock-screen lines", () => {
  it("lists what's coming, the weather, the music and the mail", () => {
    const lines = lockLines(
      {
        next: { title: "Standup", start: "2026-10-18T09:20:00", allDay: false } as never,
        weather: { status: "ready", temperature: 23.6, condition: "Clear" } as never,
        media: { status: "playing", title: "Song", artist: "Band", mock: false } as never,
        unread: 3,
      },
      now,
    );
    expect(lines).toEqual(["Next: Standup · in 20 min", "24° · Clear", "♪ Song — Band", "3 unread mails"]);
  });

  it("leaves out what isn't there", () => {
    expect(lockLines({ next: null, weather: null, media: null, unread: 0 }, now)).toEqual([]);
  });
});
