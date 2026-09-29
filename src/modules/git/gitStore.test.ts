import { describe, expect, it } from "vitest";

import { activityCells, changeSummary, timeAgo, trackedChanges } from "./gitStore";

const counts = (staged: number, modified: number, untracked: number, conflicted = 0) => ({
  staged,
  modified,
  untracked,
  conflicted,
});

describe("change text", () => {
  it("summarizes counts", () => {
    expect(changeSummary(counts(0, 0, 0))).toBe("No changes");
    expect(changeSummary(counts(0, 1, 0))).toBe("1 changed file");
    expect(changeSummary(counts(1, 9, 2))).toBe("12 changed files");
  });

  it("counts only tracked files for commit --all", () => {
    expect(trackedChanges(counts(1, 9, 2))).toBe(10);
  });
});

it("scales heatmap levels to the busiest day", () => {
  const cells = activityCells([
    { date: "2026-09-27", count: 0 },
    { date: "2026-09-28", count: 2 },
    { date: "2026-09-29", count: 8 },
  ]);
  expect(cells.map((c) => c.level)).toEqual([0, 1, 4]);
  expect(activityCells([{ date: "2026-09-29", count: 0 }])[0].level).toBe(0);
});

it("describes how long ago", () => {
  const now = Date.parse("2026-09-29T12:00:00Z");
  expect(timeAgo("2026-09-29T11:59:30Z", now)).toBe("just now");
  expect(timeAgo("2026-09-29T11:55:00Z", now)).toBe("5m ago");
  expect(timeAgo("2026-09-29T09:00:00Z", now)).toBe("3h ago");
  expect(timeAgo("2026-09-27T12:00:00Z", now)).toBe("2d ago");
  expect(timeAgo("2026-05-01T12:00:00Z", now)).toBe("5mo ago");
  expect(timeAgo("2011-01-26T19:06:43Z", now)).toBe("15y ago");
});
