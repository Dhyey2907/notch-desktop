import { describe, expect, it } from "vitest";

import { briefingDue, briefingLines, greeting } from "./briefing";

const at = (h: number, m = 0) => new Date(2026, 9, 18, h, m);
const event = (title: string, h: number, allDay = false) =>
  ({ title, start: at(h).toISOString(), allDay }) as never;

describe("morning briefing", () => {
  it("shows once a day, after 5 am, when you're at the PC", () => {
    expect(briefingDue(at(4, 59), null, "2026-10-18", true)).toBe(false);
    expect(briefingDue(at(7), null, "2026-10-18", true)).toBe(true);
    expect(briefingDue(at(7), "2026-10-18", "2026-10-18", true)).toBe(false);
    expect(briefingDue(at(7), "2026-10-17", "2026-10-18", false)).toBe(false);
  });

  it("sums up the weather, meetings, tasks and mail", () => {
    const lines = briefingLines(
      {
        weather: { status: "ready", temperature: 21.4, condition: "Partly cloudy" } as never,
        events: [event("Review", 15), event("Standup", 10), event("Holiday", 0, true)],
        openTasks: 3,
        unread: 1,
      },
      at(8),
    );
    expect(lines[0]).toBe("21° partly cloudy");
    expect(lines[1]).toMatch(/^Standup at .+ \(\+1 more\)$/);
    expect(lines.slice(2)).toEqual(["3 open tasks", "1 unread mail"]);
  });

  it("says when the day is free", () => {
    expect(briefingLines({ weather: null, events: [], openTasks: 0, unread: 0 }, at(8))).toEqual([
      "No meetings today",
    ]);
  });

  it("greets by the time of day", () => {
    expect(greeting(at(9))).toBe("Good morning");
    expect(greeting(at(13))).toBe("Good afternoon");
  });
});
