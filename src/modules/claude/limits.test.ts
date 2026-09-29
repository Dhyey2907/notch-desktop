import { describe, expect, it } from "vitest";

import { limitTone, resetText, updatedText } from "./limits";

const now = new Date("2026-09-29T12:00:00Z").getTime();
const at = (ms: number) => new Date(now + ms).toISOString();
const MIN = 60_000;

describe("plan limit formatting", () => {
  it("counts down to resets within a day", () => {
    expect(resetText(at(29 * MIN), now)).toBe("Resets in 29 min");
    expect(resetText(at(20_000), now)).toBe("Resets in 1 min");
    expect(resetText(at(89 * MIN), now)).toBe("Resets in 1 hr 29 min");
    expect(resetText(at(120 * MIN), now)).toBe("Resets in 2 hr");
    expect(resetText(at(-MIN), now)).toBe("Resetting now");
  });

  it("names the day for later resets", () => {
    expect(resetText(at(3 * 24 * 60 * MIN), now)).toMatch(/^Resets \S+/);
    expect(resetText(at(3 * 24 * 60 * MIN), now)).not.toContain("in");
  });

  it("says how fresh the numbers are", () => {
    expect(updatedText(at(-10_000), now)).toBe("Updated just now");
    expect(updatedText(at(-3 * MIN), now)).toBe("Updated 3 min ago");
    expect(updatedText(at(-125 * MIN), now)).toBe("Updated 2 hr ago");
    expect(updatedText(at(-3 * 24 * 60 * MIN), now)).toMatch(/^Updated \S+/);
  });

  it("warms up the color as the limit fills", () => {
    expect(limitTone(10)).toBe("info");
    expect(limitTone(70)).toBe("warning");
    expect(limitTone(100)).toBe("danger");
  });
});
