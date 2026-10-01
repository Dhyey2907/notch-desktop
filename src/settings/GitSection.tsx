import { useEffect, useState } from "react";

import { eventBus } from "../core/eventBus";
import type { GithubAlertSettings } from "../core/generated/GithubAlertSettings";
import type { GitSnapshot } from "../core/generated/GitSnapshot";
import type { GithubStatus } from "../core/generated/GithubStatus";
import { ipc, isIpcError } from "../core/ipc";
import { Button, ConfirmButton, Pill, Surface, Switch } from "../design/components";
import { useSettings } from ".";
import { pageLead } from "./searchIndex";
import { Section } from "./Section";
import { ProviderButton } from "./SignInWith";

function useGitSnapshot() {
  const [snapshot, setSnapshot] = useState<GitSnapshot | null>(null);
  useEffect(() => {
    ipc.gitGetSnapshot().then(setSnapshot, () => setSnapshot(null));
    return eventBus.on("GitChanged", ({ event }) => setSnapshot(event.payload));
  }, []);
  return snapshot;
}

function useGithubStatus() {
  const [status, setStatus] = useState<GithubStatus | null>(null);
  useEffect(() => {
    ipc.githubStatus().then(setStatus, () => setStatus(null));
    return eventBus.on("GithubChanged", ({ event }) => setStatus(event.payload));
  }, []);
  return status;
}

function GithubAccount() {
  const status = useGithubStatus();
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  if (!status || (!status.available && !status.signedIn)) return null;

  const start = async () => {
    setError(null);
    try {
      await ipc.githubSignIn();
    } catch (e) {
      setError(isIpcError(e) ? e.message.replace(/^invalid input: /, "") : "Couldn't start GitHub sign-in.");
    }
  };
  const copy = async (code: string) => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2000);
    } catch {
      // Clipboard can be unavailable; the code is on screen to type.
    }
  };

  return (
    <Surface>
      <div className="settings__row">
        <span className="settings__row-text">
          <span className="settings__row-label">GitHub</span>
          <span className="settings__row-description">
            {status.signedIn
              ? `Signed in${status.login ? ` as @${status.login}` : ""}. Pull requests come straight from GitHub.`
              : "Sign in to see pull requests without installing the GitHub CLI."}
          </span>
        </span>
        {status.signedIn && <Pill tone="success">Signed in</Pill>}
      </div>
      {status.signedIn && <GithubAlertSwitches />}
      {status.signedIn ? (
        <div className="settings__buttons">
          <ConfirmButton
            variant="danger"
            confirmLabel="Sign out of GitHub?"
            onConfirm={() => void ipc.githubSignOut()}
          >
            Sign out
          </ConfirmButton>
        </div>
      ) : status.pending ? (
        <>
          <p className="settings__lead">
            On the GitHub page that just opened, enter this code and approve Notch Desktop:
          </p>
          <p className="github-code" aria-label="GitHub sign-in code">
            {status.pending.userCode}
          </p>
          <div className="settings__buttons settings__buttons--start">
            <Button variant="primary" onClick={() => void copy(status.pending!.userCode)}>
              {copied ? "Copied" : "Copy code"}
            </Button>
            <Button onClick={() => void ipc.openExternal(status.pending!.verificationUri)}>
              Open GitHub again
            </Button>
            <Button onClick={() => void ipc.githubCancelSignIn()}>Cancel</Button>
          </div>
          <p className="settings__note">Waiting for you to approve on github.com…</p>
        </>
      ) : (
        <>
          <div className="settings__buttons settings__buttons--start">
            <ProviderButton provider="github" onClick={() => void start()} />
          </div>
          {(error ?? status.lastError) && (
            <p className="settings__note settings__note--warning">{error ?? status.lastError}</p>
          )}
        </>
      )}
    </Surface>
  );
}

export function GitSection() {
  const snapshot = useGitSnapshot();
  const active = snapshot?.repo?.path;

  return (
    <Section title="Git" lead={pageLead("git")}>
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
      <GithubAccount />
      <Surface>
        <p className="settings__lead">
          Commits, pushes, and pulls use your normal Git setup and credentials, and always ask you to confirm
          first. Pulls only fast-forward, so nothing is merged or rewritten behind your back. Pull requests
          come from your GitHub sign-in above, or else from the GitHub CLI (<strong>gh</strong>) if it's
          installed and signed in.
        </p>
      </Surface>
    </Section>
  );
}

function GithubAlertSwitches() {
  const alerts = useSettings((s) => s.settings?.githubAlerts);
  if (!alerts) return null;
  const set = (change: Partial<GithubAlertSettings>) =>
    void useSettings.getState().update((s) => ({ ...s, githubAlerts: { ...s.githubAlerts, ...change } }));
  return (
    <>
      <p className="settings__lead">Pop up in the notch when, on GitHub:</p>
      <Switch
        label="Someone requests your review"
        checked={!alerts.hideReviews}
        onChange={(on) => set({ hideReviews: !on })}
      />
      <Switch
        label="A workflow you started fails"
        checked={!alerts.hideCiFailures}
        onChange={(on) => set({ hideCiFailures: !on })}
      />
      <Switch
        label="A workflow you started succeeds"
        checked={alerts.showCiPasses}
        onChange={(on) => set({ showCiPasses: on })}
      />
      <Switch
        label="You're mentioned or assigned"
        checked={!alerts.hideMentions}
        onChange={(on) => set({ hideMentions: !on })}
      />
      <p className="settings__note">Everything also waits in the Git module's Inbox.</p>
    </>
  );
}
