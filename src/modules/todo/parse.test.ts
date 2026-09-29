import { describe, expect, it } from "vitest";

import { formatCountdown, formatMinutes, parseQuickAdd } from "./parse";

describe("quick add", () => {
  it("takes a trailing duration as the timer", () => {
    expect(parseQuickAdd("Write report 25m")).toEqual({ title: "Write report", minutes: 25 });
    expect(parseQuickAdd("Tea 4 min")).toEqual({ title: "Tea", minutes: 4 });
    expect(parseQuickAdd("Study 1h")).toEqual({ title: "Study", minutes: 60 });
    expect(parseQuickAdd("Study 1h 30m")).toEqual({ title: "Study", minutes: 90 });
    expect(parseQuickAdd("Study 1h30m")).toEqual({ title: "Study", minutes: 90 });
    expect(parseQuickAdd("Deep work 2 hours 15 minutes")).toEqual({ title: "Deep work", minutes: 135 });
    expect(parseQuickAdd("  Walk 90 minutes  ")).toEqual({ title: "Walk", minutes: 90 });
  });

  it("leaves ordinary text alone", () => {
    expect(parseQuickAdd("Buy milk")).toEqual({ title: "Buy milk" });
    expect(parseQuickAdd("Call mom at 5")).toEqual({ title: "Call mom at 5" });
    expect(parseQuickAdd("Read chapter 25")).toEqual({ title: "Read chapter 25" });
    expect(parseQuickAdd("Room 42m2 measurements")).toEqual({ title: "Room 42m2 measurements" });
  });

  it("needs a title and a sensible length", () => {
    expect(parseQuickAdd("25m")).toEqual({ title: "25m" });
    expect(parseQuickAdd("Nap 0m")).toEqual({ title: "Nap 0m" });
    expect(parseQuickAdd("Marathon 30h")).toEqual({ title: "Marathon 30h" });
  });
});

describe("formatting", () => {
  it("writes durations and countdowns", () => {
    expect(formatMinutes(25)).toBe("25 min");
    expect(formatMinutes(60)).toBe("1 h");
    expect(formatMinutes(95)).toBe("1 h 35 min");
    expect(formatCountdown(245_000)).toBe("4:05");
    expect(formatCountdown(754_000)).toBe("12:34");
    expect(formatCountdown(3_723_000)).toBe("1:02:03");
    expect(formatCountdown(-5000)).toBe("0:00");
    expect(formatCountdown(400)).toBe("0:01");
  });
});
