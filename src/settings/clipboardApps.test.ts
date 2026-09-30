import { describe, expect, it } from "vitest";

import { exeName } from "./clipboardApps";

describe("clipboard block list", () => {
  it("normalises what people type to a program name", () => {
    expect(exeName("Slack")).toBe("slack.exe");
    expect(exeName("  Chrome.EXE ")).toBe("chrome.exe");
    expect(exeName("C:/Program Files/App/Tool.exe")).toBe("tool.exe");
    expect(exeName("   ")).toBe("");
  });
});
