import { describe, expect, it } from "vitest";

import { defaultPicks, LAYOUTS } from "./layouts";

describe("snap layouts", () => {
  it("each layout's zones stay on the screen", () => {
    for (const layout of LAYOUTS)
      for (const z of layout.zones) {
        expect(z.x + z.w).toBeLessThanOrEqual(1.0000001);
        expect(z.y + z.h).toBeLessThanOrEqual(1.0000001);
      }
  });

  it("guesses the newest windows for the zones", () => {
    expect(defaultPicks([7, 8, 9], 2)).toEqual([7, 8]);
    expect(defaultPicks([7], 3)).toEqual([7, null, null]);
  });
});
