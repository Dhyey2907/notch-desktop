import { describe, expect, it } from "vitest";

import type { AppearanceSettings } from "../core/generated/AppearanceSettings";
import { appearanceVars, islandBackground, readableOn, restingSize, scaledRadius } from "./appearance";

const LOOK: AppearanceSettings = {
  size: "normal",
  textSize: "normal",
  bubble: "pill",
  bubbleClock: false,
  theme: "black",
  opacity: 100,
  roundness: 100,
  accent: "",
  accentFromMusic: false,
  accentFromWallpaper: false,
  offsetX: 0,
  offsetY: 0,
  edge: "top",
  offsetSide: 160,
  fadeWhileTyping: false,
  lockscreenInfo: false,
};

describe("appearance", () => {
  it("keeps the stylesheet's look by default", () => {
    const vars = appearanceVars(LOOK);
    expect(vars["--island-bg"]).toBe("rgba(9, 9, 9, 1)");
    expect(vars["--text-md"]).toBe("13px");
    expect(vars["--active"]).toBeUndefined();
  });

  it("makes glass see-through even at full opacity", () => {
    expect(islandBackground({ theme: "glass", opacity: 100 })).toBe("rgba(22, 22, 28, 0.75)");
    expect(islandBackground({ theme: "graphite", opacity: 60 })).toBe("rgba(36, 36, 40, 0.6)");
  });

  it("scales text and picks readable text on the accent", () => {
    const vars = appearanceVars({ ...LOOK, textSize: "large", accent: "#1e3a8a" });
    expect(vars["--text-md"]).toBe("14.6px");
    expect(vars["--active"]).toBe("#1e3a8a");
    expect(vars["--on-active"]).toBe("#ffffff");
    expect(readableOn("#fde047")).toBe("#0a0a0a");
  });

  it("sizes the bubble and corners", () => {
    expect(restingSize(LOOK)).toEqual({ width: 64, height: 14 });
    expect(restingSize({ bubble: "pill", bubbleClock: true }).width).toBe(104);
    expect(restingSize({ bubble: "dot", bubbleClock: true })).toEqual({ width: 22, height: 12 });
    expect(scaledRadius(24, 50)).toBe(12);
    expect(scaledRadius(24, 0)).toBe(4);
  });
});
