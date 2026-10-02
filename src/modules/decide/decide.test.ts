import { describe, expect, it } from "vitest";

import { parseOptions, randomBelow, wheelRotation } from "./decide";

describe("decide", () => {
  it("picks without modulo bias", () => {
    // 2^32 - 1 is in the biased tail for n = 3 and is skipped.
    const values = [0xffff_ffff, 7];
    const source = (out: Uint32Array<ArrayBuffer>) => {
      out[0] = values.shift() ?? 0;
    };
    expect(randomBelow(3, source)).toBe(1);
    expect(randomBelow(0)).toBe(0);
    for (let i = 0; i < 50; i++) expect(randomBelow(6)).toBeLessThan(6);
  });

  it("reads the wheel's options", () => {
    expect(parseOptions("pizza, Sushi,\n tacos ,, sushi")).toEqual(["pizza", "Sushi", "tacos"]);
    expect(parseOptions(Array.from({ length: 20 }, (_, i) => `o${i}`).join(","))).toHaveLength(12);
  });

  it("lands the wheel on the chosen slice", () => {
    // 4 slices of 90°: slice 1 is centred at 135°, so the wheel turns to 225° (+ full turns).
    expect(wheelRotation(0, 1, 4) % 360).toBe(225);
    const next = wheelRotation(1845, 0, 4);
    expect(next % 360).toBe(315);
    expect(next).toBeGreaterThan(1845);
  });
});
