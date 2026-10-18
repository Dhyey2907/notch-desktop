import { describe, expect, it } from "vitest";

import { moveIndex } from "./grid";

describe("window switcher grid", () => {
  it("wraps left and right, stops at the top and bottom", () => {
    expect(moveIndex(0, "ArrowLeft", 6)).toBe(5);
    expect(moveIndex(5, "ArrowRight", 6)).toBe(0);
    expect(moveIndex(1, "ArrowDown", 6)).toBe(5);
    expect(moveIndex(2, "ArrowDown", 6)).toBe(2);
    expect(moveIndex(1, "ArrowUp", 6)).toBe(1);
    expect(moveIndex(0, "ArrowRight", 0)).toBe(0);
  });
});
