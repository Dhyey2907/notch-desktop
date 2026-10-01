import { describe, expect, it } from "vitest";

import { ENTRIES, pageFromId, PAGES, searchSettings, settingAnchor } from "./searchIndex";
import { prettyShortcut, shortcutFrom } from "./shortcutText";

describe("settings search", () => {
  it("finds settings by label and by everyday words", () => {
    expect(searchSettings("accent")[0].label).toBe("Accent color");
    expect(searchSettings("do not disturb").map((r) => r.label)).toContain("Quiet hours");
    expect(searchSettings("fahrenheit")[0].page).toBe("home");
    expect(searchSettings("   ")).toEqual([]);
    expect(searchSettings("zzzz")).toEqual([]);
  });

  it("ranks label matches above keyword matches", () => {
    const results = searchSettings("clipboard");
    expect(results[0].label).toBe("Clipboard history");
  });

  it("points every entry at a real page and keeps old links working", () => {
    for (const entry of ENTRIES) expect(PAGES.some((p) => p.id === entry.page)).toBe(true);
    expect(pageFromId("appearance")).toBe("look");
    expect(pageFromId("modules")).toBe("dock");
    expect(pageFromId("nope")).toBe("general");
    expect(settingAnchor("Show the notch on")).toBe("setting-show-the-notch-on");
  });
});

describe("shortcut text", () => {
  const key = (
    code: string,
    mods: Partial<Record<"ctrlKey" | "altKey" | "shiftKey" | "metaKey", boolean>>,
  ) => ({
    code,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    metaKey: false,
    ...mods,
  });

  it("records combinations the backend understands", () => {
    expect(shortcutFrom(key("KeyN", { ctrlKey: true, altKey: true }))).toBe("Ctrl+Alt+N");
    expect(shortcutFrom(key("Space", { ctrlKey: true, shiftKey: true }))).toBe("Ctrl+Shift+Space");
    expect(shortcutFrom(key("F9", {}))).toBe("F9");
    expect(shortcutFrom(key("Digit1", { metaKey: true, altKey: true }))).toBe("Alt+Super+1");
  });

  it("refuses keys that would fire while typing", () => {
    expect(shortcutFrom(key("KeyA", {}))).toBeNull();
    expect(shortcutFrom(key("KeyA", { shiftKey: true }))).toBeNull();
    expect(shortcutFrom(key("ControlLeft", { ctrlKey: true }))).toBeNull();
  });

  it("shows shortcuts nicely", () => {
    expect(prettyShortcut("Ctrl+Alt+N")).toBe("Ctrl + Alt + N");
    expect(prettyShortcut("Super+ArrowUp")).toBe("Win + Up");
  });
});
