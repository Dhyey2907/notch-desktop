import { describe, expect, it } from "vitest";

import type { CalendarEvent } from "../../core/generated/CalendarEvent";
import type { CalendarSnapshot } from "../../core/generated/CalendarSnapshot";
import { countdownText, eventsOn, isStale, monthGrid, nextEvent, relativeStart } from "./calendarStore";

const local = (h: number, m = 0, day = 28) => new Date(2026, 8, day, h, m);
function ev(id: string, start: Date, minutes = 30, allDay = false): CalendarEvent {
  return {
    id,
    title: id,
    start: start.toISOString(),
    end: new Date(start.getTime() + minutes * 60_000).toISOString(),
    allDay,
  };
}

describe("eventsOn", () => {
  it("returns the day's events with all-day first", () => {
    const events = [
      ev("late", local(15)),
      ev("early", local(9)),
      ev("allday", local(0), 24 * 60, true),
      ev("tomorrow", local(9, 0, 29)),
    ];
    expect(eventsOn(events, local(12)).map((e) => e.id)).toEqual(["allday", "early", "late"]);
  });

  it("includes events that started yesterday and run into today", () => {
    const overnight = ev("overnight", local(23, 0, 27), 120);
    expect(eventsOn([overnight], local(12)).map((e) => e.id)).toEqual(["overnight"]);
  });
});

describe("nextEvent", () => {
  it("skips started and all-day events", () => {
    const now = local(10).getTime();
    const events = [
      ev("past", local(9)),
      ev("allday", local(0), 1440, true),
      ev("b", local(14)),
      ev("a", local(11)),
    ];
    expect(nextEvent(events, now)?.id).toBe("a");
    expect(nextEvent([], now)).toBeNull();
  });
});

describe("text", () => {
  it("describes how soon an event starts", () => {
    const now = local(10).getTime();
    expect(relativeStart(ev("x", local(10, 12)), now)).toBe("in 12 min");
    expect(relativeStart(ev("x", local(12, 5)), now)).toBe("in 2 h 5 min");
    expect(relativeStart(ev("x", local(11)), now)).toBe("in 1 h");
    expect(relativeStart(ev("x", local(9)), now)).toBe("now");
  });

  it("counts down to the start", () => {
    expect(countdownText(272_000)).toBe("Starts in 4:32");
    expect(countdownText(500)).toBe("Starts in 0:01");
    expect(countdownText(0)).toBe("Started");
  });
});

it("flags stale data", () => {
  const snapshot = {
    status: { state: "connected" },
    events: [],
    mock: false,
    lastSyncedAt: new Date(0).toISOString(),
  } as CalendarSnapshot;
  expect(isStale(snapshot, 29 * 60_000)).toBe(false);
  expect(isStale(snapshot, 31 * 60_000)).toBe(true);
  expect(isStale({ ...snapshot, lastSyncedAt: undefined }, 1e12)).toBe(false);
});

it("lays out a month Monday-first", () => {
  const weeks = monthGrid(new Date(2026, 8, 15)); // September 2026 starts on a Tuesday
  expect(weeks[0][0]).toBeNull();
  expect(weeks[0][1]?.getDate()).toBe(1);
  expect(weeks.flat().filter(Boolean)).toHaveLength(30);
  expect(weeks.every((w) => w.length === 7)).toBe(true);
});
