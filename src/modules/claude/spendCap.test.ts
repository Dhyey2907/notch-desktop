import { describe, expect, it } from "vitest";

import { capStep, newStep } from "./spendCap";

describe("Claude spend cap", () => {
  it("steps at 80% and at the cap", () => {
    expect(capStep(7.9, 10)).toBe(0);
    expect(capStep(8, 10)).toBe(80);
    expect(capStep(10, 10)).toBe(100);
    expect(capStep(50, 0)).toBe(0);
  });

  it("warns only when the step goes up", () => {
    expect(newStep(80, 0)).toBe(true);
    expect(newStep(80, 80)).toBe(false);
    expect(newStep(100, 80)).toBe(true);
    expect(newStep(0, 100)).toBe(false);
  });
});
