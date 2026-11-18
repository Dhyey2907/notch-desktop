import { afterEach, describe, expect, it } from "vitest";

import {
  edgeGap,
  islandCorners,
  setNotchPlacement,
  SHOULDER,
  TUCKED_STRIP,
  tuckOffset,
  windowSizeFor,
  WINDOW_MARGIN,
} from "./geometry";

const BUBBLE = { width: 64, height: 14 };

describe("notch placement", () => {
  afterEach(() => setNotchPlacement("top", true));

  it("hangs from the top edge with square top corners and room for its shoulders", () => {
    setNotchPlacement("top", true);
    expect(edgeGap()).toBe(0);
    expect(islandCorners(24)).toBe("0px 0px 24px 24px");
    expect(windowSizeFor("collapsed", BUBBLE)).toEqual({
      width: BUBBLE.width + SHOULDER * 2,
      height: BUBBLE.height,
    });
  });

  it("floats with all corners round and a gap when it has a distance from the edge", () => {
    setNotchPlacement("top", false);
    expect(edgeGap()).toBe(WINDOW_MARGIN.top);
    expect(islandCorners(24)).toBe("24px 24px 24px 24px");
    expect(windowSizeFor("collapsed", BUBBLE)).toEqual({
      width: BUBBLE.width,
      height: BUBBLE.height + WINDOW_MARGIN.top,
    });
  });

  it("squares the corners against whichever edge it is on", () => {
    setNotchPlacement("bottom", true);
    expect(islandCorners(10)).toBe("10px 10px 0px 0px");
    setNotchPlacement("left", true);
    expect(islandCorners(10)).toBe("0px 10px 10px 0px");
    // No shoulders on the sides.
    expect(windowSizeFor("collapsed", BUBBLE).width).toBe(BUBBLE.width);
  });

  it("shrinks to a thin strip along the edge while tucked away", () => {
    expect(windowSizeFor("collapsed", BUBBLE, 0, { tucked: true })).toEqual({
      width: BUBBLE.width + TUCKED_STRIP.slack * 2,
      height: TUCKED_STRIP.thickness,
    });
    expect(windowSizeFor("collapsed", BUBBLE, 0, { tucked: true, vertical: true })).toEqual({
      width: TUCKED_STRIP.thickness,
      height: BUBBLE.height + TUCKED_STRIP.slack * 2,
    });
    // Only the resting bubble tucks.
    expect(windowSizeFor("expanded", undefined, 0, { tucked: true })).toEqual(windowSizeFor("expanded"));
  });

  it("slides into the edge it sits on", () => {
    expect(tuckOffset(BUBBLE).y).toBeLessThan(-BUBBLE.height);
    setNotchPlacement("right", true);
    expect(tuckOffset(BUBBLE).x).toBeGreaterThan(BUBBLE.width);
  });
});
