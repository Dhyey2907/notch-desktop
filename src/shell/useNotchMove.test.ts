import { describe, expect, it } from "vitest";

import { spotPatch } from "./useNotchMove";

describe("dragging the notch", () => {
  it("saves the place along the top or bottom as a sideways offset", () => {
    expect(spotPatch({ edge: "bottom", along: -120 })).toEqual({ edge: "bottom", offsetX: -120 });
  });

  it("saves the place on a side as a height", () => {
    expect(spotPatch({ edge: "right", along: 300 })).toEqual({ edge: "right", offsetSide: 300 });
  });
});
