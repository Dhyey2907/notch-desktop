import { describe, expect, it } from "vitest";

import type { AppearanceSettings } from "../core/generated/AppearanceSettings";
import {
  appearanceVars,
  islandBackground,
  readableOn,
  restingRadius,
  restingSize,
  scaledRadius,
} from "./appearance";

const LOOK: AppearanceSettings = {
  size: "normal",
  textSize: "normal",
  bubble: "pill",
  shape: "pill",
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

  it("keeps glass a readable frosted tint at any opacity", () => {
    expect(islandBackground({ theme: "glass", opacity: 100 })).toBe("rgba(22, 22, 28, 0.96)");
    expect(islandBackground({ theme: "glass", opacity: 40 })).toBe("rgba(22, 22, 28, 0.86)");
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
  });

  it("gives each notch shape its own size and corners", () => {
    expect(restingSize({ bubble: "pill", bubbleClock: false, shape: "notch" })).toEqual({
      width: 124,
      height: 26,
    });
    expect(restingSize({ bubble: "pill", bubbleClock: true, shape: "rounded" }).width).toBe(128);
    expect(restingRadius({ bubble: "pill", shape: "pill" })).toBe(Infinity);
    expect(restingRadius({ bubble: "pill", shape: "sharp" })).toBeLessThan(
      restingRadius({ bubble: "pill", shape: "notch" }),
    );
    expect(scaledRadius(24, 50)).toBe(12);
    expect(scaledRadius(24, 0)).toBe(4);
  });
});
