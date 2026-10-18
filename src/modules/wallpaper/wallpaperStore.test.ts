import { describe, expect, it } from "vitest";

import type { WallpaperState } from "../../core/generated/WallpaperState";
import {
  effectiveTarget,
  isCurrent,
  itemsFor,
  picturesOnly,
  screenOptions,
  shownTab,
  targetLabel,
} from "./wallpaperStore";

const pic = (path: string, saved = true) => ({ path, name: path, saved });

const state = (over: Partial<WallpaperState> = {}): WallpaperState => ({
  saved: [pic("C:\\w\\a.jpg"), pic("C:\\w\\b.jpg")],
  folder: [pic("D:\\p\\c.png", false)],
  recent: [],
  screens: [
    { id: "mon-1", label: "Screen 1", primary: true, current: "C:\\w\\a.jpg" },
    { id: "mon-2", label: "Screen 2", primary: false, current: "D:\\p\\c.png" },
  ],
  target: "all",
  ...over,
});

describe("wallpaper store", () => {
  it("opens on Recent only when something was set before", () => {
    expect(shownTab(null, state())).toBe("saved");
    expect(shownTab(null, state({ recent: [pic("C:\\w\\a.jpg")] }))).toBe("recent");
    expect(shownTab("folder", state())).toBe("folder");
  });

  it("lists each tab's pictures", () => {
    expect(itemsFor(state(), "saved")).toHaveLength(2);
    expect(itemsFor(state(), "folder")[0].path).toBe("D:\\p\\c.png");
    expect(itemsFor(null, "saved")).toEqual([]);
  });

  it("offers each screen plus All, or just the one screen", () => {
    expect(screenOptions(state()).map((o) => o.label)).toEqual(["Screen 1", "Screen 2", "All"]);
    expect(screenOptions(state({ screens: [state().screens[0]] })).map((o) => o.label)).toEqual(["Screen 1"]);
  });

  it("falls back to every screen when the chosen one is unplugged", () => {
    expect(effectiveTarget(state({ target: "mon-2" }))).toBe("mon-2");
    expect(effectiveTarget(state({ target: "gone" }))).toBe("all");
  });

  it("marks the current wallpaper for the chosen screen(s)", () => {
    expect(isCurrent(state({ target: "mon-1" }), "c:\\W\\A.JPG")).toBe(true);
    expect(isCurrent(state({ target: "mon-2" }), "C:\\w\\a.jpg")).toBe(false);
    // On All, it must be on every screen.
    expect(isCurrent(state(), "C:\\w\\a.jpg")).toBe(false);
  });

  it("names where it went", () => {
    expect(targetLabel(state(), "mon-2")).toBe("Screen 2");
    expect(targetLabel(state(), "all")).toBe("every screen");
  });

  it("keeps only pictures from a drop", () => {
    expect(picturesOnly(["a.JPG", "b.txt", "c.webp", "d.gif"])).toEqual(["a.JPG", "c.webp"]);
  });
});
