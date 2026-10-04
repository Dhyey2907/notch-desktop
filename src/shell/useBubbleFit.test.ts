import { describe, expect, it } from "vitest";

import { bubbleWidthFor } from "./useBubbleFit";

describe("bubble width", () => {
  it("stays small with little to show and grows to fit more", () => {
    expect(bubbleWidthFor([])).toBe(64);
    expect(bubbleWidthFor([9])).toBe(64);
    // Music, Claude, a download and the pet: 9+9+10+13 plus 3 gaps of 6, plus the ends.
    expect(bubbleWidthFor([9, 9, 10, 13])).toBe(41 + 18 + 18);
    expect(bubbleWidthFor([0, 9, 0])).toBe(64);
    expect(bubbleWidthFor(Array(30).fill(20))).toBe(160);
  });
});
