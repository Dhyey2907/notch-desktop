import { describe, expect, it } from "vitest";

import { currentLineIndex } from "./lyrics";

const lines = [
  { atMs: 5_000, text: "one" },
  { atMs: 9_000, text: "two" },
  { atMs: 12_500, text: "three" },
];

describe("lyrics", () => {
  it("finds the line being sung", () => {
    expect(currentLineIndex(lines, 0)).toBe(-1);
    expect(currentLineIndex(lines, 5_000)).toBe(0);
    expect(currentLineIndex(lines, 11_999)).toBe(1);
    expect(currentLineIndex(lines, 60_000)).toBe(2);
    expect(currentLineIndex([], 1_000)).toBe(-1);
  });
});
