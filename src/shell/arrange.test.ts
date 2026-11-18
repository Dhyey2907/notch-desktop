import { describe, expect, it } from "vitest";

import { nearestUnit, savedOrder } from "./arrange";

describe("drag to rearrange", () => {
  it("finds the card nearest the pointer in a grid", () => {
    const grid = [
      { x: 100, y: 50 },
      { x: 300, y: 50 },
      { x: 100, y: 150 },
      { x: 300, y: 150 },
    ];
    expect(nearestUnit({ x: 290, y: 140 }, grid)).toBe(3);
    expect(nearestUnit({ x: 120, y: 60 }, grid)).toBe(0);
    expect(nearestUnit({ x: 0, y: 0 }, [])).toBe(-1);
  });

  it("saves the new order first and keeps hidden sections after it", () => {
    const units = [{ ids: ["weather"] }, { ids: ["network", "battery"] }, { ids: ["system"] }];
    expect(savedOrder(units, ["system", "network", "battery", "weather", "screen"])).toEqual([
      "weather",
      "network",
      "battery",
      "system",
      "screen",
    ]);
  });
});
