// GitHub-first views for the Git module (when signed in with GitHub): pick one of your
// repositories, then see its latest commits and open pull requests — no local folder needed.
// Linking a local clone adds branch, changes, commit, push, and pull.
import { GitCommitHorizontal, Lock, Search } from "lucide-react";
import { useEffect, useMemo, useState, type ReactNode } from "react";

import type { GithubRepo } from "../../core/generated/GithubRepo";
import type { GithubRepoOverview } from "../../core/generated/GithubRepoOverview";
import { ipc, isIpcError } from "../../core/ipc";
import { Button, EmptyState, ErrorState, ModuleContainer, SegmentedControl } from "../../design/components";
import { timeAgo } from "./gitStore";
import { PrItems } from "./PrItems";

const REFRESH_MS = 5 * 60_000;

/** Current time, re-read every minute (for "3h ago" labels). */
function useMinuteNow() {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = window.setInterval(() => setNow(Date.now()), 60_000);
    return () => window.clearInterval(id);
  }, []);
  return now;
}

const errorText = (error: unknown) =>
  isIpcError(error) ? error.message.replace(/^invalid input: /, "") : "Couldn't reach GitHub. Try again.";

export function GithubRepoPicker({
  onPicked,
  onLocalFolder,
  onCancel,
}: {
  onPicked: () => void;
  onLocalFolder: () => void;
  onCancel?: () => void;
}) {
  const [repos, setRepos] = useState<GithubRepo[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const now = useMinuteNow();

  useEffect(() => {
    let alive = true;
    ipc.githubRepositories().then(
      (list) => alive && setRepos(list),
      (e) => alive && setError(errorText(e)),
    );
    return () => {
      alive = false;
    };
  }, []);

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (repos ?? []).filter((r) => !q || r.nameWithOwner.toLowerCase().includes(q));
  }, [repos, query]);

  const pick = async (name: string) => {
    setBusy(name);
    try {
      await ipc.githubSelectRepo(name);
      onPicked();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(null);
    }
  };

  const trailing = (
    <>
      <Button onClick={onLocalFolder}>Local folder…</Button>
      {onCancel && <Button onClick={onCancel}>Cancel</Button>}
    </>
  );

  return (
    <ModuleContainer title="Choose a repository" trailing={trailing}>
      {error ? (
        <ErrorState title="Couldn't load your repositories" body={error} />
      ) : !repos ? (
        <p className="git__muted git__loading">Loading your GitHub repositories…</p>
      ) : (
        <div className="gh-picker">
          <label className="gh-picker__search">
            <Search size={14} aria-hidden />
            <input
              aria-label="Search your repositories"
              placeholder="Search your repositories"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </label>
          {shown.length === 0 ? (
            <p className="git__muted">No repositories match.</p>
          ) : (
            <ul className="git-prs" aria-label="Your GitHub repositories">
              {shown.map((repo) => (
                <li key={repo.nameWithOwner}>
                  <button
                    type="button"
                    className="git-prs__row"
                    disabled={busy !== null}
                    onClick={() => void pick(repo.nameWithOwner)}
                  >
                    {repo.isPrivate && <Lock size={12} aria-label="Private" />}
                    <span className="git-prs__title" title={repo.description ?? repo.nameWithOwner}>
                      {repo.nameWithOwner}
                    </span>
                    <span className="git__muted">
                      {busy === repo.nameWithOwner
                        ? "Opening…"
                        : repo.pushedAt
                          ? timeAgo(repo.pushedAt, now)
                          : ""}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </ModuleContainer>
  );
}

type Tab = "commits" | "prs";

export function GithubRepoView({
  nameWithOwner,
  onChange,
  onLinkFolder,
  extra,
}: {
  nameWithOwner: string;
  onChange: () => void;
  onLinkFolder: () => void;
  /** More trailing controls (the inbox button). */
  extra?: ReactNode;
}) {
  const [overview, setOverview] = useState<GithubRepoOverview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<Tab>("commits");
  const now = useMinuteNow();

  useEffect(() => {
    let alive = true;
    const load = () =>
      ipc.githubRepoOverview(nameWithOwner).then(
        (o) => {
          if (!alive) return;
          setOverview(o);
          setError(null);
        },
        (e) => alive && setError(errorText(e)),
      );
    void load();
    const id = window.setInterval(() => void load(), REFRESH_MS);
    return () => {
      alive = false;
      window.clearInterval(id);
    };
  }, [nameWithOwner]);

  const trailing = (
    <>
      {extra}
      <Button onClick={onChange}>Change</Button>
      <Button onClick={onLinkFolder}>Link local folder</Button>
      <SegmentedControl<Tab>
        label="GitHub view"
        value={tab}
        options={[
          { value: "commits", label: "Commits" },
          { value: "prs", label: "Pull requests" },
        ]}
        onChange={setTab}
      />
    </>
  );

  let body;
  if (error && !overview) {
    body = <ErrorState title="Couldn't load this repository" body={error} />;
  } else if (!overview) {
    body = <p className="git__muted git__loading">Loading from GitHub…</p>;
  } else if (tab === "prs") {
    body = <PrItems prs={overview.pullRequests} />;
  } else if (overview.commits.length === 0) {
    body = (
      <EmptyState title="No commits yet" body="Push something to this repository and it shows up here." />
    );
  } else {
    body = (
      <ul className="git-prs" aria-label="Latest commits">
        {overview.commits.map((c) => (
          <li key={c.url}>
            <button type="button" className="git-prs__row" onClick={() => void ipc.openExternal(c.url)}>
              <GitCommitHorizontal size={14} aria-hidden />
              <span className="git-prs__title" title={c.headline}>
                {c.headline}
              </span>
              <span className="git__muted">
                {c.author} · {timeAgo(c.at, now)}
              </span>
            </button>
          </li>
        ))}
      </ul>
    );
  }

  return (
    <ModuleContainer title={nameWithOwner.split("/")[1] ?? "GitHub"} trailing={trailing}>
      {overview?.defaultBranch && tab === "commits" && (
        <p className="git__muted gh-branch">
          {nameWithOwner} · {overview.defaultBranch}
        </p>
      )}
      {body}
    </ModuleContainer>
  );
}
