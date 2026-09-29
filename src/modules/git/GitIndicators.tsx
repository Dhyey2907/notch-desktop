import { ArrowUp, GitBranch, LoaderCircle } from "lucide-react";

import { MockBadge } from "../../design/components";
import { changeSummary, useGit } from "./gitStore";
import "./git.css";

/** Collapsed: only speaks up while an operation runs or commits are waiting to be pushed. */
export function GitCompact() {
  const running = useGit((s) => s.snapshot?.running);
  const ahead = useGit((s) => (s.snapshot?.status.state === "ready" ? s.snapshot.branch.ahead : 0));
  if (running) {
    return (
      <span className="git-compact" role="img" aria-label={`Git ${running} in progress`}>
        <LoaderCircle className="git-compact__spin" size={12} aria-hidden />
      </span>
    );
  }
  if (ahead > 0) {
    return (
      <span className="git-compact" role="img" aria-label={`${ahead} commits not pushed`}>
        <ArrowUp size={12} aria-hidden />
        {ahead}
      </span>
    );
  }
  return null;
}

export function GitPeek() {
  const snapshot = useGit((s) => s.snapshot);
  if (!snapshot?.repo || snapshot.status.state !== "ready") {
    return (
      <div className="module-peek">
        <span className="module-peek__title">Git</span>
        <span className="module-peek__meta">Click to choose a repository</span>
      </div>
    );
  }
  const { branch } = snapshot;
  return (
    <div className="module-peek">
      <span className="module-peek__title">
        <GitBranch size={14} aria-hidden /> {snapshot.repo.name} / {branch.branch ?? "detached"}{" "}
        {snapshot.mock && <MockBadge />}
      </span>
      <span className="module-peek__meta">
        {changeSummary(branch.changes)}
        {branch.ahead > 0 && ` · ↑${branch.ahead}`}
        {branch.behind > 0 && ` · ↓${branch.behind}`}
      </span>
    </div>
  );
}
