import { describe, expect, it } from "vitest";

import { formatClock, formatTemperature } from "./clock";

describe("clock", () => {
  it("formats 12 and 24 hour times", () => {
    const evening = new Date(2026, 9, 1, 18, 5);
    const midnight = new Date(2026, 9, 1, 0, 30);
    expect(formatClock(evening, false)).toBe("6:05 pm");
    expect(formatClock(evening, false, false)).toBe("6:05");
    expect(formatClock(midnight, false)).toBe("12:30 am");
    expect(formatClock(evening, true)).toBe("18:05");
    expect(formatClock(midnight, true)).toBe("00:30");
  });

  it("converts temperatures", () => {
    expect(formatTemperature(21, false)).toBe("21°C");
    expect(formatTemperature(21, true)).toBe("70°F");
    expect(formatTemperature(-40, true)).toBe("-40°F");
  });
});
