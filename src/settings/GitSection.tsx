import { useEffect, useState } from "react";

import { eventBus } from "../core/eventBus";
import type { GitSnapshot } from "../core/generated/GitSnapshot";
import { ipc } from "../core/ipc";
import { Button, Pill, Surface } from "../design/components";
import { Section } from "./Section";

function useGitSnapshot() {
  const [snapshot, setSnapshot] = useState<GitSnapshot | null>(null);
  useEffect(() => {
    ipc.gitGetSnapshot().then(setSnapshot, () => setSnapshot(null));
    return eventBus.on("GitChanged", ({ event }) => setSnapshot(event.payload));
  }, []);
  return snapshot;
}

export function GitSection() {
  const snapshot = useGitSnapshot();
  const active = snapshot?.repo?.path;

  return (
    <Section title="Git">
      <Surface>
        <div className="settings__row">
          <span className="settings__row-text">
            <span className="settings__row-label">Repositories</span>
            <span className="settings__row-description">
              The notch shows one at a time; switch between these from the Git module.
            </span>
          </span>
          <Button onClick={() => void ipc.gitPickRepo().catch(() => {})}>Add repository</Button>
        </div>
        {snapshot?.status.state === "gitMissing" && (
          <p className="settings__note settings__note--warning">
            Git isn't installed. Get it from git-scm.com, then reopen Notch Desktop.
          </p>
        )}
        {snapshot && snapshot.recentRepos.length === 0 ? (
          <p className="settings__note">No repositories yet.</p>
        ) : (
          <ul className="repo-list">
            {snapshot?.recentRepos.map((repo) => (
              <li key={repo.path} className="repo-list__item">
                <span className="settings__row-text">
                  <span className="settings__row-label">
                    {repo.name} {repo.path === active && <Pill tone="info">Showing</Pill>}
                  </span>
                  <span className="settings__row-description">{repo.path}</span>
                </span>
                {repo.path !== active && (
                  <Button onClick={() => void ipc.gitSelectRepo(repo.path).catch(() => {})}>Show</Button>
                )}
                <Button onClick={() => void ipc.gitForgetRepo(repo.path)}>Remove</Button>
              </li>
            ))}
          </ul>
        )}
      </Surface>
      <Surface>
        <p className="settings__lead">
          Commits, pushes, and pulls use your normal Git setup and credentials, and always ask you to confirm
          first. Pulls only fast-forward, so nothing is merged or rewritten behind your back. Pull requests
          come from the GitHub CLI (<strong>gh</strong>) if it's installed and signed in.
        </p>
      </Surface>
    </Section>
  );
}
