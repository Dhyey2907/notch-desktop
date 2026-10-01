// New GitHub notifications as cards in the notch: review requests, CI results, mentions.
import type { ActivityInput } from "../../activity/activityStore";
import type { GithubAlertSettings } from "../../core/generated/GithubAlertSettings";
import type { GithubNotification } from "../../core/generated/GithubNotification";
import type { NotificationKind } from "../../core/generated/NotificationKind";
import type { Tone } from "../../design/components";

const HEADINGS: Record<NotificationKind, string> = {
  reviewRequested: "Review requested",
  ciFailed: "CI failed",
  ciPassed: "CI passed",
  mention: "You were mentioned",
  assigned: "Assigned to you",
  other: "GitHub",
};

const TONES: Record<NotificationKind, Tone> = {
  reviewRequested: "info",
  ciFailed: "danger",
  ciPassed: "success",
  mention: "info",
  assigned: "info",
  other: "neutral",
};

export function heading(kind: NotificationKind): string {
  return HEADINGS[kind];
}

/** Whether this kind pops up, given the user's settings (everything shows in the inbox). */
export function shouldAlert(kind: NotificationKind, settings: GithubAlertSettings | undefined): boolean {
  switch (kind) {
    case "reviewRequested":
      return !settings?.hideReviews;
    case "ciFailed":
      return !settings?.hideCiFailures;
    case "ciPassed":
      return Boolean(settings?.showCiPasses);
    case "mention":
    case "assigned":
      return !settings?.hideMentions;
    default:
      return false;
  }
}

export function alertCard(
  n: GithubNotification,
  run: { open: (url: string) => Promise<void>; markRead: (id: string) => Promise<void> },
): ActivityInput {
  return {
    moduleId: "git",
    priority: "normal",
    tone: TONES[n.kind],
    dedupKey: `github-${n.id}`,
    countRepeats: false,
    // Long enough to read a PR title; hovering holds it.
    timeoutMs: 9000,
    title: HEADINGS[n.kind],
    body: `${n.repo} · ${n.title}`,
    actions: [
      { id: "read", label: "Mark read", variant: "secondary", run: () => run.markRead(n.id) },
      { id: "open", label: "Open", variant: "primary", run: () => run.open(n.url) },
    ],
  };
}
