import { describe, expect, it, vi } from "vitest";

import { calendarUrl, localDate, mapsUrl, suggestionCard } from "./suggestions";

describe("copy suggestions", () => {
  it("reads the backend's local time", () => {
    const d = localDate("2026-10-01T16:30:00");
    expect([d.getFullYear(), d.getMonth(), d.getDate(), d.getHours(), d.getMinutes()]).toEqual([
      2026, 9, 1, 16, 30,
    ]);
  });

  it("prefills a Google Calendar event", () => {
    const url = new URL(calendarUrl("Dentist", localDate("2026-10-12T09:30:00"), false, "Asia/Kolkata"));
    expect(url.origin + url.pathname).toBe("https://calendar.google.com/calendar/render");
    expect(url.searchParams.get("text")).toBe("Dentist");
    expect(url.searchParams.get("dates")).toBe("20261012T093000/20261012T103000");
    expect(url.searchParams.get("ctz")).toBe("Asia/Kolkata");
    const allDay = new URL(calendarUrl("Holiday", localDate("2026-12-31T09:00:00"), true, "Asia/Kolkata"));
    expect(allDay.searchParams.get("dates")).toBe("20261231/20270101");
  });

  it("links an address to Maps", () => {
    expect(mapsUrl("221B Baker Street, London")).toBe(
      "https://www.google.com/maps/search/?api=1&query=221B%20Baker%20Street%2C%20London",
    );
  });

  it("offers reminder and calendar for a date, maps for an address", async () => {
    const run = {
      remind: vi.fn(() => Promise.resolve()),
      open: vi.fn(() => Promise.resolve()),
      timeZone: "UTC",
    };
    const card = suggestionCard(
      { text: "meeting Thu 4pm", remindAt: "2026-10-01T16:00:00", allDay: false },
      run,
    )!;
    expect(card.actions?.map((a) => a.id)).toEqual(["calendar", "remind"]);
    await card.actions![1].run();
    expect(run.remind).toHaveBeenCalledWith("meeting Thu 4pm", localDate("2026-10-01T16:00:00").getTime());
    const place = suggestionCard({ text: "x", allDay: false, address: "221B Baker Street, London" }, run)!;
    await place.actions![0].run();
    expect(run.open).toHaveBeenCalledWith(mapsUrl("221B Baker Street, London"));
    expect(suggestionCard({ text: "x", allDay: false }, run)).toBeNull();
  });
});
