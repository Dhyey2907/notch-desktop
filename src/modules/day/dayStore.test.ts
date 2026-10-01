import { describe, expect, it } from "vitest";

import { dueIn, planLine } from "./dayStore";

describe("day", () => {
  const now = new Date(2026, 9, 1, 12, 0).getTime();
  const plus = (h: number) => new Date(now + h * 3_600_000).toISOString();

  it("says when work is due", () => {
    expect(dueIn(plus(5), now)).toBe("in 5 h");
    expect(dueIn(plus(0.25), now)).toBe("in 15 min");
    expect(dueIn(plus(28), now)).toBe("tomorrow");
    expect(dueIn(plus(-3), now)).toBe("3 h ago");
  });

  it("splits plan lines into time and task", () => {
    expect(planLine("09:30–10:30 Write report")).toEqual({ time: "09:30–10:30", text: "Write report" });
    expect(planLine("- 14:00 - 14:15: Break")).toEqual({ time: "14:00-14:15", text: "Break" });
    expect(planLine("Focus: ship the release")).toEqual({ text: "Focus: ship the release" });
    expect(planLine("23:00 Sleep")).toEqual({ time: "23:00", text: "Sleep" });
  });
});
