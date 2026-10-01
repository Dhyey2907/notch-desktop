import { describe, expect, it, vi } from "vitest";

import type { GithubNotification } from "../../core/generated/GithubNotification";
import { alertCard, shouldAlert } from "./githubAlerts";

const note = (overrides: Partial<GithubNotification> = {}): GithubNotification => ({
  id: "42",
  kind: "reviewRequested",
  title: "Add login",
  repo: "octo/app",
  url: "https://github.com/octo/app/pull/12",
  updatedAt: "2026-10-01T10:00:00Z",
  ...overrides,
});

describe("GitHub alerts", () => {
  it("pops up reviews, failures and mentions by default, but not passes", () => {
    expect(shouldAlert("reviewRequested", undefined)).toBe(true);
    expect(shouldAlert("ciFailed", undefined)).toBe(true);
    expect(shouldAlert("mention", undefined)).toBe(true);
    expect(shouldAlert("assigned", undefined)).toBe(true);
    expect(shouldAlert("ciPassed", undefined)).toBe(false);
    expect(shouldAlert("other", undefined)).toBe(false);
  });

  it("follows the settings", () => {
    const settings = { hideReviews: true, hideCiFailures: false, showCiPasses: true, hideMentions: true };
    expect(shouldAlert("reviewRequested", settings)).toBe(false);
    expect(shouldAlert("ciPassed", settings)).toBe(true);
    expect(shouldAlert("mention", settings)).toBe(false);
  });

  it("opens the page or marks the thread read", async () => {
    const run = { open: vi.fn(() => Promise.resolve()), markRead: vi.fn(() => Promise.resolve()) };
    const card = alertCard(note({ kind: "ciFailed", title: "CI failed on main" }), run);
    expect(card.title).toBe("CI failed");
    expect(card.tone).toBe("danger");
    expect(card.body).toBe("octo/app · CI failed on main");
    await card.actions!.find((a) => a.id === "open")!.run();
    expect(run.open).toHaveBeenCalledWith("https://github.com/octo/app/pull/12");
    await card.actions!.find((a) => a.id === "read")!.run();
    expect(run.markRead).toHaveBeenCalledWith("42");
  });
});
