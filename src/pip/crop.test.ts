import { describe, expect, it } from "vitest";

import { applyCrop, FULL, selectionCrop } from "./crop";

describe("picture-in-picture crop", () => {
  it("reads a box dragged in any direction", () => {
    expect(selectionCrop({ x0: 0.8, y0: 0.9, x1: 0.2, y1: 0.1 })).toEqual({
      x: 0.2,
      y: 0.1,
      w: 0.6000000000000001,
      h: 0.8,
    });
  });

  it("crops inside the current crop", () => {
    const first = applyCrop(FULL, { x: 0.5, y: 0, w: 0.5, h: 0.5 });
    expect(first).toEqual({ x: 0.5, y: 0, w: 0.5, h: 0.5 });
    expect(applyCrop(first, { x: 0.5, y: 0.5, w: 0.5, h: 0.5 })).toEqual({
      x: 0.75,
      y: 0.25,
      w: 0.25,
      h: 0.25,
    });
  });

  it("ignores a click without a drag", () => {
    expect(applyCrop(FULL, { x: 0.4, y: 0.4, w: 0, h: 0 })).toBe(FULL);
  });
});
