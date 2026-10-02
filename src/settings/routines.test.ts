import { describe, expect, it } from "vitest";

import { stepLabel } from "./routines";

describe("routine steps", () => {
  it("reads like a sentence", () => {
    expect(stepLabel({ kind: "app", path: "C:/Program Files/Code/Code.exe" })).toBe("Open Code");
    expect(stepLabel({ kind: "url", url: "https://www.github.com/" })).toBe("Open github.com");
    expect(stepLabel({ kind: "folder", path: "C:/dev/notch-desktop" })).toBe("Open folder notch-desktop");
    expect(stepLabel({ kind: "close", exe: "Discord.exe" })).toBe("Close Discord");
    expect(stepLabel({ kind: "focus", minutes: 50 })).toBe("Focus for 50 min");
  });
});
