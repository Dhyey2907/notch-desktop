import { describe, expect, it } from "vitest";

import { dayKeyBack, dayLabel, formatMinutes, greeting } from "./dashboard";

describe("home dashboard", () => {
  it("greets by time of day", () => {
    expect(greeting(8)).toBe("Good morning");
    expect(greeting(13)).toBe("Good afternoon");
    expect(greeting(19)).toBe("Good evening");
    expect(greeting(23)).toBe("Good night");
    expect(greeting(3)).toBe("Good night");
  });

  it("formats screen time", () => {
    expect(formatMinutes(378)).toBe("6h 18m");
    expect(formatMinutes(42)).toBe("42m");
    expect(formatMinutes(120)).toBe("2h");
    expect(formatMinutes(0)).toBe("0m");
  });
});

describe("screen time history", () => {
  it("names and keys past days", () => {
    const now = new Date(2026, 9, 4, 15);
    expect(dayLabel(now, 0)).toBe("Today");
    expect(dayLabel(now, 1)).toBe("Yesterday");
    expect(dayKeyBack(now, 0)).toBe("2026-10-04");
    expect(dayKeyBack(now, 4)).toBe("2026-09-30");
  });
});
