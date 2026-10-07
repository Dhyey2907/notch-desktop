import { describe, expect, it } from "vitest";

import { findWhen, parseCapture } from "./parse";

// Wednesday 7 Oct 2026, 10:00 local.
const NOW = new Date(2026, 9, 7, 10, 0);
const local = (d: number, h: number, m = 0) => new Date(2026, 9, d, h, m).getTime();

describe("quick capture", () => {
  it("files reminders with their time", () => {
    expect(parseCapture("remind me at 5 to call mom", NOW)).toEqual({
      kind: "reminder",
      title: "call mom",
      at: local(7, 17),
    });
    expect(parseCapture("call mom tomorrow 9:30am", NOW)).toEqual({
      kind: "reminder",
      title: "call mom",
      at: local(8, 9, 30),
    });
    expect(parseCapture("pay rent friday", NOW)).toMatchObject({
      kind: "reminder",
      title: "pay rent",
      at: local(9, 9),
    });
    expect(parseCapture("stretch in 20 min", NOW)).toMatchObject({
      kind: "reminder",
      title: "stretch",
      at: NOW.getTime() + 20 * 60_000,
    });
    // No time yet: still a reminder, the time gets picked after.
    expect(parseCapture("remind me to water plants", NOW)).toEqual({
      kind: "reminder",
      title: "water plants",
    });
  });

  it("spots notes, events, links and timers", () => {
    expect(parseCapture("note: the wifi password is on the fridge", NOW)).toEqual({
      kind: "note",
      title: "the wifi password is on the fridge",
    });
    expect(parseCapture("lunch with Ann friday 1pm", NOW)).toEqual({
      kind: "event",
      title: "lunch with Ann",
      at: local(9, 13),
    });
    expect(parseCapture("event: Board review", NOW)).toMatchObject({ kind: "event", title: "Board review" });
    expect(parseCapture("https://example.com/post", NOW)).toEqual({
      kind: "link",
      title: "https://example.com/post",
      url: "https://example.com/post",
    });
    expect(parseCapture("read later https://x.org/a.", NOW)).toMatchObject({
      kind: "link",
      url: "https://x.org/a",
    });
    expect(parseCapture("tea 4m", NOW)).toEqual({ kind: "task", title: "tea", minutes: 4 });
    expect(parseCapture("buy milk", NOW)).toEqual({ kind: "task", title: "buy milk" });
    expect(parseCapture("task: lunch with team at 1", NOW)).toMatchObject({ kind: "task" });
  });

  it("reads clock times sensibly", () => {
    expect(findWhen("at 9", NOW)?.at).toBe(local(7, 21)); // 9 am has passed → 9 pm
    expect(findWhen("at 11", NOW)?.at).toBe(local(7, 11));
    expect(findWhen("8am", NOW)?.at).toBe(local(8, 8)); // passed today → tomorrow
    expect(findWhen("17:45", NOW)?.at).toBe(local(7, 17, 45));
    expect(findWhen("tonight", NOW)?.at).toBe(local(7, 20));
    expect(findWhen("wednesday noon", NOW)?.at).toBe(local(14, 12)); // today is Wednesday → next week
    expect(findWhen("buy 2 apples", NOW)).toBeNull();
    expect(findWhen("at 25:00", NOW)).toBeNull();
  });
});
