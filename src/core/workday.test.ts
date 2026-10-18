import { describe, expect, it } from "vitest";

import { span, weekendIn, workday } from "./workday";

const DAY = {
  startMin: 540,
  endMin: 1080,
  workDays: [1, 2, 3, 4, 5],
  noAutoPlan: false,
  noWorkdayBar: false,
  shareGoogle: false,
  reviewAt: 1260,
  noReview: false,
  noBriefing: false,
};
// Thursday 1 October 2026.
const at = (d: number, h: number, m = 0) => new Date(2026, 9, d, h, m);

describe("workday", () => {
  it("knows where you are in the day", () => {
    expect(workday(DAY, at(1, 8))).toEqual({ phase: "before", fraction: 0, minutes: 60 });
    const mid = workday(DAY, at(1, 13, 30));
    expect(mid.phase).toBe("during");
    expect(mid.fraction).toBeCloseTo(0.5);
    expect(mid.minutes).toBe(270);
    expect(workday(DAY, at(1, 19)).phase).toBe("after");
    expect(workday(DAY, at(3, 12)).phase).toBe("off"); // Saturday
  });

  it("counts down to the weekend", () => {
    // Thursday 17:00 → Friday 18:00 = 25 h.
    expect(weekendIn(DAY, at(1, 17))).toBe(25 * 60);
    expect(weekendIn(DAY, at(2, 17))).toBe(60); // Friday
    expect(weekendIn(DAY, at(3, 12))).toBeNull(); // Saturday
    expect(weekendIn({ ...DAY, workDays: [1, 2, 3, 4, 5, 6, 7] }, at(1, 12))).toBeNull();
  });

  it("writes spans", () => {
    expect(span(45)).toBe("45m");
    expect(span(200)).toBe("3h 20m");
    expect(span(25 * 60)).toBe("1d 1h");
  });
});
