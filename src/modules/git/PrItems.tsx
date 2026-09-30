import { GitPullRequest } from "lucide-react";

import type { PullRequests } from "../../core/generated/PullRequests";
import { ipc } from "../../core/ipc";
import { EmptyState, Pill } from "../../design/components";

/** Open pull requests, or why they can't be shown. */
export function PrItems({ prs }: { prs: PullRequests }) {
  if (prs.state === "unavailable") {
    const copy = {
      ghMissing: [
        "GitHub CLI not found",
        "Install the GitHub CLI (cli.github.com) to see pull requests here.",
      ],
      notAuthenticated: [
        "Sign in to GitHub",
        "Use Continue with GitHub in Settings → Git (or run  gh auth login  in a terminal).",
      ],
      notGithub: ["Not a GitHub repository", "This repository has no GitHub remote."],
      failed: ["Couldn't load pull requests", "GitHub didn't respond. Try again in a moment."],
    }[prs.reason];
    return <EmptyState title={copy[0]} body={copy[1]} />;
  }
  if (prs.items.length === 0) {
    return <EmptyState title="No open pull requests" body="Nothing waiting on review in this repository." />;
  }
  const review = (decision: string) =>
    decision === "APPROVED" ? (
      <Pill tone="success">Approved</Pill>
    ) : decision === "CHANGES_REQUESTED" ? (
      <Pill tone="warning">Changes requested</Pill>
    ) : decision === "REVIEW_REQUIRED" ? (
      <Pill>Review required</Pill>
    ) : null;

  return (
    <ul className="git-prs">
      {prs.items.map((pr) => (
        <li key={pr.number}>
          <button type="button" className="git-prs__row" onClick={() => void ipc.openExternal(pr.url)}>
            <GitPullRequest size={14} aria-hidden />
            <span className="git-prs__number">#{pr.number}</span>
            <span className="git-prs__title" title={pr.title}>
              {pr.title}
            </span>
            {pr.isDraft ? <Pill>Draft</Pill> : review(pr.reviewDecision)}
            <span className="git__muted">{pr.author}</span>
          </button>
        </li>
      ))}
    </ul>
  );
}
