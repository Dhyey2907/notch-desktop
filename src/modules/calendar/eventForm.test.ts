import { describe, expect, it } from "vitest";

import type { CalendarEvent } from "../../core/generated/CalendarEvent";
import { buildEvent, dateInput, nextHalfHour, waitingInvites } from "./eventForm";

describe("new event form", () => {
  it("builds timed and all-day events", () => {
    const timed = buildEvent(" Dentist ", "2026-10-08", "14:30", 45, false, " ")!;
    expect(timed.title).toBe("Dentist");
    expect(new Date(timed.start).getHours()).toBe(14);
    expect(Date.parse(timed.end) - Date.parse(timed.start)).toBe(45 * 60_000);
    expect(timed.location).toBeUndefined();

    const allDay = buildEvent("Trip", "2026-10-08", "", 0, true, "Goa")!;
    expect(new Date(allDay.start).getDate()).toBe(8);
    expect(new Date(allDay.end).getDate()).toBe(9);
    expect(allDay.location).toBe("Goa");

    expect(buildEvent("x", "", "10:00", 30, false, "")).toBeNull();
    expect(buildEvent("x", "2026-10-08", "", 30, false, "")).toBeNull();
  });

  it("suggests the next half hour and formats dates", () => {
    expect(nextHalfHour(new Date(2026, 9, 7, 9, 10))).toBe("09:30");
    expect(nextHalfHour(new Date(2026, 9, 7, 9, 40))).toBe("10:00");
    expect(dateInput(new Date(2026, 0, 5))).toBe("2026-01-05");
  });

  it("lists invites waiting for an answer", () => {
    const e = (id: string, start: string, myResponse?: CalendarEvent["myResponse"]): CalendarEvent => ({
      id,
      title: id,
      start,
      end: new Date(Date.parse(start) + 3600_000).toISOString(),
      allDay: false,
      myResponse,
    });
    const now = Date.parse("2026-10-07T08:00:00Z");
    const list = waitingInvites(
      [
        e("later", "2026-10-09T10:00:00Z", "needsAction"),
        e("mine", "2026-10-07T10:00:00Z"),
        e("soon", "2026-10-07T10:00:00Z", "needsAction"),
        e("done", "2026-10-07T11:00:00Z", "accepted"),
        e("past", "2026-10-06T10:00:00Z", "needsAction"),
      ],
      now,
    );
    expect(list.map((x) => x.id)).toEqual(["soon", "later"]);
  });
});
