import { FolderOpen, GitBranch } from "lucide-react";
import { useEffect, useState } from "react";

import type { GitOperation } from "../../core/generated/GitOperation";
import type { GitSnapshot } from "../../core/generated/GitSnapshot";
import type { PullRequests } from "../../core/generated/PullRequests";
import { ipc, isIpcError } from "../../core/ipc";
import {
  Button,
  ConfirmButton,
  EmptyState,
  ErrorState,
  Heatmap,
  IconButton,
  MockBadge,
  ModuleContainer,
  Pill,
  SegmentedControl,
  Switch,
  TextField,
} from "../../design/components";
import { activityCells, changeSummary, timeAgo, trackedChanges, useGit } from "./gitStore";
import { GithubInbox, InboxButton } from "./GithubInbox";
import { GithubRepoPicker, GithubRepoView } from "./GithubView";
import { useGithub } from "./githubStore";
import { notify } from "./notify";
import { PrItems } from "./PrItems";
import { useSettings } from "../../settings";
import "./git.css";

type Tab = "overview" | "prs";
/** While the PR tab is open, refresh this often (and on repo changes). */
const PR_REFRESH_MS = 5 * 60_000;

const errorText = (error: unknown) =>
  isIpcError(error) && error.kind === "invalid_input"
    ? error.message.replace(/^invalid input: /, "")
    : "Something went wrong. Try again.";

const pickRepo = () => void ipc.gitPickRepo().catch(() => {});

/** Current time, re-read every `intervalMs` (for "3h ago" labels). */
function useNow(intervalMs: number) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = window.setInterval(() => setNow(Date.now()), intervalMs);
    return () => window.clearInterval(id);
  }, [intervalMs]);
  return now;
}

export default function GitExpanded() {
  const snapshot = useGit((s) => s.snapshot);
  const github = useGithub((s) => s.status);
  const githubRepo = useSettings((s) => s.settings?.git.githubRepo);
  const [tab, setTab] = useState<Tab>("overview");
  // Showing the GitHub repository picker (from the switcher or "Change").
  const [choosing, setChoosing] = useState(false);
  // Showing the GitHub notifications inbox.
  const [inbox, setInbox] = useState(false);

  if (!snapshot) return <ModuleContainer title="Git">{null}</ModuleContainer>;

  if (github?.signedIn && inbox) return <GithubInbox onClose={() => setInbox(false)} />;
  const inboxButton = github?.signedIn ? <InboxButton onOpen={() => setInbox(true)} /> : null;

  const noLocalRepo = snapshot.status.state === "noRepo" || !snapshot.repo;
  // Signed in with GitHub: pick from your GitHub repositories instead of a folder.
  if (github?.signedIn && (choosing || (noLocalRepo && !githubRepo))) {
    return (
      <GithubRepoPicker
        onPicked={() => setChoosing(false)}
        onLocalFolder={() => {
          setChoosing(false);
          pickRepo();
        }}
        onCancel={choosing ? () => setChoosing(false) : undefined}
      />
    );
  }
  if (github?.signedIn && noLocalRepo && githubRepo) {
    return (
      <GithubRepoView
        nameWithOwner={githubRepo}
        onChange={() => setChoosing(true)}
        onLinkFolder={pickRepo}
        extra={inboxButton}
      />
    );
  }

  if (snapshot.status.state === "gitMissing") {
    return (
      <ModuleContainer title="Git">
        <EmptyState
          title="Git isn't installed"
          body="Install Git for Windows (git-scm.com), then reopen Overline to see your repositories here."
        />
      </ModuleContainer>
    );
  }
  if (snapshot.status.state === "noRepo" || !snapshot.repo) {
    return (
      <ModuleContainer title="Git">
        <EmptyState
          title="Your code, at a glance."
          body={
            github?.available
              ? "Choose a repository folder on this PC — or sign in with GitHub (Settings → Git) to pick from your GitHub repositories."
              : "Choose a repository to see its branch, changes, and activity — and commit, push, or pull from here."
          }
          action={
            <Button variant="primary" onClick={pickRepo}>
              Choose repository
            </Button>
          }
        />
      </ModuleContainer>
    );
  }

  const trailing = (
    <>
      {snapshot.mock && <MockBadge />}
      {inboxButton}
      <RepoSwitcher snapshot={snapshot} onGithub={github?.signedIn ? () => setChoosing(true) : undefined} />
      <SegmentedControl<Tab>
        label="Git view"
        value={tab}
        options={[
          { value: "overview", label: "Overview" },
          { value: "prs", label: "Pull requests" },
        ]}
        onChange={setTab}
      />
    </>
  );

  return (
    <ModuleContainer title="Git" trailing={trailing}>
      {snapshot.status.state === "error" ? (
        <ErrorState
          title="Repository unavailable"
          body={snapshot.status.message}
          action={<Button onClick={pickRepo}>Choose another</Button>}
        />
      ) : tab === "overview" ? (
        <Overview snapshot={snapshot} />
      ) : (
        // Keyed by repo so switching repositories starts from a fresh "loading" state.
        <PullRequestList key={snapshot.repo.path} />
      )}
    </ModuleContainer>
  );
}

function RepoSwitcher({ snapshot, onGithub }: { snapshot: GitSnapshot; onGithub?: () => void }) {
  const current = snapshot.repo?.path ?? "";
  return (
    <select
      className="git__repo-select"
      aria-label="Repository"
      value={current}
      onChange={(e) => {
        const value = e.currentTarget.value;
        if (value === "__pick__") pickRepo();
        else if (value === "__github__") onGithub?.();
        else if (value !== current) void ipc.gitSelectRepo(value).catch(() => {});
      }}
    >
      {snapshot.recentRepos.map((r) => (
        <option key={r.path} value={r.path}>
          {r.name}
        </option>
      ))}
      {onGithub && <option value="__github__">Your GitHub repositories…</option>}
      <option value="__pick__">Choose another folder…</option>
    </select>
  );
}

function Overview({ snapshot }: { snapshot: GitSnapshot }) {
  const { branch, commits, activity, running } = snapshot;
  const [composing, setComposing] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  const now = useNow(60_000);
  const busy = running != null;

  const run = async (operation: GitOperation, message?: string, includeUntracked?: boolean) => {
    setNote(null);
    try {
      await ipc.gitRun(operation, message, includeUntracked);
      const done = { commit: "Committed", push: "Pushed", pull: "Pulled" }[operation];
      notify({
        priority: "low",
        title: `${done} · ${snapshot.repo?.name}`,
        body: branch.branch ?? undefined,
        tone: "success",
      });
      return true;
    } catch (error) {
      const text = errorText(error);
      setNote(text);
      notify({ priority: "normal", title: `Git ${operation} failed`, body: text, tone: "danger" });
      return false;
    }
  };

  if (composing) {
    return (
      <CommitComposer
        snapshot={snapshot}
        onCancel={() => setComposing(false)}
        onCommit={async (message, includeUntracked) => {
          if (await run("commit", message, includeUntracked)) setComposing(false);
        }}
      />
    );
  }

  return (
    <div className="git">
      <div className="git__main">
        <p className="git__branch">
          <GitBranch size={16} aria-hidden />
          <span className="git__repo">{snapshot.repo?.name}</span>
          <span className="git__slash">/</span>
          <span>{branch.branch ?? "detached HEAD"}</span>
        </p>
        <p className="git__stats">
          {changeSummary(branch.changes)}
          {branch.changes.conflicted > 0 && <Pill tone="danger">{branch.changes.conflicted} conflicts</Pill>}
          {branch.upstream ? (
            <>
              {branch.ahead > 0 && <Pill tone="info">↑ {branch.ahead} to push</Pill>}
              {branch.behind > 0 && <Pill tone="warning">↓ {branch.behind} to pull</Pill>}
              {branch.ahead === 0 && branch.behind === 0 && <span className="git__muted">Up to date</span>}
            </>
          ) : (
            branch.branch && <span className="git__muted">Not published yet</span>
          )}
        </p>
        {commits.length > 0 && (
          <ol className="git__commits" aria-label="Recent commits">
            {commits.slice(0, 3).map((c) => (
              <li key={c.hash} className="git__last" title={c.subject}>
                <code>{c.short}</code> {c.subject}{" "}
                <span className="git__muted">· {timeAgo(c.time, now)}</span>
              </li>
            ))}
          </ol>
        )}
        <div className="git__actions">
          <Button
            disabled={
              busy || branch.changes.staged + branch.changes.modified + branch.changes.untracked === 0
            }
            onClick={() => setComposing(true)}
          >
            {running === "commit" ? "Committing…" : "Commit"}
          </Button>
          <ConfirmButton
            disabled={busy || !branch.branch || (branch.upstream != null && branch.ahead === 0)}
            confirmLabel={
              branch.upstream ? `Push ${branch.ahead} to ${branch.upstream}?` : "Publish branch to origin?"
            }
            onConfirm={() => void run("push")}
          >
            {running === "push" ? "Pushing…" : branch.upstream ? "Push" : "Publish"}
          </ConfirmButton>
          <ConfirmButton
            disabled={busy || !branch.upstream}
            confirmLabel={`Pull from ${branch.upstream ?? "origin"}?`}
            onConfirm={() => void run("pull")}
          >
            {running === "pull" ? "Pulling…" : "Pull"}
          </ConfirmButton>
          <IconButton
            icon={FolderOpen}
            label="Open repository folder"
            onClick={() => void ipc.gitOpenFolder()}
          />
        </div>
        {note && <p className="git__note">{note}</p>}
      </div>
      <div className="git__activity">
        <p className="git__label">Your commits · last 26 weeks</p>
        <Heatmap cells={activityCells(activity)} label="Your commits over the last 26 weeks" unit="commits" />
      </div>
    </div>
  );
}

function CommitComposer({
  snapshot,
  onCancel,
  onCommit,
}: {
  snapshot: GitSnapshot;
  onCancel: () => void;
  onCommit: (message: string, includeUntracked: boolean) => void;
}) {
  const { changes } = snapshot.branch;
  const [message, setMessage] = useState("");
  const [includeUntracked, setIncludeUntracked] = useState(false);
  const [writing, setWriting] = useState(false);
  const [aiError, setAiError] = useState<string | null>(null);
  const count = trackedChanges(changes) + (includeUntracked ? changes.untracked : 0);
  const busy = snapshot.running != null;

  // Claude Code reads the changes (on this PC) and suggests a message to edit.
  const suggest = async () => {
    setWriting(true);
    setAiError(null);
    try {
      setMessage(await ipc.gitSuggestMessage(includeUntracked));
    } catch (e) {
      setAiError(isIpcError(e) ? e.message.replace(/^invalid input: /, "") : "Couldn't write a message.");
    } finally {
      setWriting(false);
    }
  };

  return (
    <div className="git-compose">
      <TextField
        label="Commit message"
        value={message}
        onChange={setMessage}
        placeholder="Describe the change"
      />
      {changes.untracked > 0 && (
        <Switch
          label={`Include ${changes.untracked} new ${changes.untracked === 1 ? "file" : "files"}`}
          description="Off: only files Git already tracks are committed."
          checked={includeUntracked}
          onChange={setIncludeUntracked}
        />
      )}
      {aiError && <p className="git__error">{aiError}</p>}
      <div className="git__actions">
        <Button onClick={() => void suggest()} disabled={busy || writing || count === 0}>
          {writing ? "Writing…" : "✨ Write with Claude"}
        </Button>
        <Button
          variant="primary"
          disabled={busy || !message.trim() || count === 0}
          onClick={() => onCommit(message, includeUntracked)}
        >
          {busy ? "Committing…" : `Commit ${count} ${count === 1 ? "file" : "files"}`}
        </Button>
        <Button onClick={onCancel} disabled={busy}>
          Cancel
        </Button>
      </div>
    </div>
  );
}

function PullRequestList() {
  const [prs, setPrs] = useState<PullRequests | null>(null);

  useEffect(() => {
    let cancelled = false;
    const load = () =>
      ipc.gitPullRequests().then(
        (result) => !cancelled && setPrs(result),
        () => !cancelled && setPrs({ state: "unavailable", reason: "failed" }),
      );
    void load();
    const id = window.setInterval(() => void load(), PR_REFRESH_MS);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, []);

  if (!prs) return <p className="git__muted git__loading">Loading pull requests…</p>;
  return <PrItems prs={prs} />;
}
