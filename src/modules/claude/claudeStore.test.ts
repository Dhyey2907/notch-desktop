import { describe, expect, it } from "vitest";

import {
  comparisonText,
  formatHour,
  formatTokens,
  limitLabel,
  prettyModel,
  primarySession,
} from "./claudeStore";

describe("formatting", () => {
  it("abbreviates token counts", () => {
    expect(formatTokens(950)).toBe("950");
    expect(formatTokens(12_400)).toBe("12.4K");
    expect(formatTokens(48_812_345)).toBe("48.8M");
    expect(formatTokens(2_000_000_000)).toBe("2B");
  });

  it("names models readably and passes unknown ids through", () => {
    expect(prettyModel("claude-opus-5-5")).toBe("Opus 5.5");
    expect(prettyModel("claude-sonnet-5")).toBe("Sonnet 5");
    expect(prettyModel("claude-haiku-4-5-20251001")).toBe("Haiku 4.5");
    expect(prettyModel("some-other-model")).toBe("some-other-model");
  });

  it("formats hours and limits", () => {
    expect(formatHour(0)).toBe("12 AM");
    expect(formatHour(15)).toBe("3 PM");
    expect(limitLabel("five_hour")).toBe("5-hour");
    expect(limitLabel("new_kind")).toBe("new kind");
  });
});

describe("comparisonText", () => {
  it("describes the change against the previous period", () => {
    expect(comparisonText(123, 100, 7)).toBe("↑ 23% tokens vs the previous 7 days");
    expect(comparisonText(95, 100, 30)).toBe("↓ 5% tokens vs the previous 30 days");
    expect(comparisonText(100, 100, 7)).toBe("Same as the previous 7 days");
    expect(comparisonText(5, 0, 7)).toBe("New activity vs the previous 7 days");
    expect(comparisonText(0, 0, 7)).toBeNull();
    expect(comparisonText(5, undefined, 7)).toBeNull();
  });
});

it("features the session that most needs attention", () => {
  const s = (id: string, state: "working" | "waiting" | "idle") => ({
    sessionId: id,
    project: id,
    state,
    updatedAt: "2026-09-29T00:00:00Z",
  });
  expect(primarySession([s("a", "idle"), s("b", "working"), s("c", "waiting")])?.sessionId).toBe("c");
  expect(primarySession([s("a", "idle"), s("b", "working")])?.sessionId).toBe("b");
  expect(primarySession([])).toBeNull();
});
