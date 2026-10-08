import { useEffect, useState } from "react";

import { eventBus } from "../core/eventBus";
import type { AuditEntry } from "../core/generated/AuditEntry";
import type { ClaudeStatus } from "../core/generated/ClaudeStatus";
import { ipc, isIpcError } from "../core/ipc";
import { Button, ConfirmButton, Pill, Surface } from "../design/components";
import { AgentsSetting } from "./AgentsSetting";
import { AssistantSetting } from "./AssistantSetting";
import { pageLead } from "./searchIndex";
import { Section } from "./Section";

function useClaudeStatus() {
  const [status, setStatus] = useState<ClaudeStatus | null>(null);
  useEffect(() => {
    ipc.claudeStatus().then(setStatus, () => setStatus(null));
    return eventBus.on("ClaudeStatusChanged", ({ event }) => setStatus(event.payload));
  }, []);
  return status;
}

function useApprovalAudit() {
  const [entries, setEntries] = useState<AuditEntry[] | null>(null);
  useEffect(() => {
    const load = () => ipc.claudeApprovalAudit(20).then(setEntries, () => setEntries(null));
    void load();
    // Pending list shrinking means something was just decided and audited.
    return eventBus.on("ApprovalsChanged", () => void load());
  }, []);
  return entries;
}

const DECISION_LABEL: Record<string, string> = { allow: "Allowed", deny: "Denied", ask: "In terminal" };
const DECIDED_BY_LABEL: Record<string, string> = {
  you: "by you",
  session_rule: "for this session",
  timeout: "no answer in time",
  app_closed: "Overline closed",
  agent_exited: "the agent closed",
  elsewhere: "answered in the agent",
};

function RecentApprovals() {
  const entries = useApprovalAudit();
  return (
    <Surface>
      <div className="settings__row">
        <span className="settings__row-text">
          <span className="settings__row-label">Recent approvals</span>
          <span className="settings__row-description">
            Permission requests answered through Overline, kept on this PC only (last 500).
          </span>
        </span>
      </div>
      {entries && entries.length > 0 ? (
        <ul className="settings__audit">
          {entries.map((e) => (
            <li key={e.id} className="settings__audit-row">
              <Pill tone={e.decision === "allow" ? "success" : e.decision === "deny" ? "danger" : "neutral"}>
                {DECISION_LABEL[e.decision] ?? e.decision}
              </Pill>
              <code className="settings__audit-code" title={e.summary}>
                {e.summary}
              </code>
              <span className="settings__audit-meta">
                {[
                  e.tool,
                  e.project,
                  DECIDED_BY_LABEL[e.decidedBy] ?? e.decidedBy,
                  new Date(e.at).toLocaleString(undefined, { dateStyle: "short", timeStyle: "short" }),
                ]
                  .filter(Boolean)
                  .join(" · ")}
              </span>
            </li>
          ))}
        </ul>
      ) : (
        <p className="settings__note">Nothing yet.</p>
      )}
    </Surface>
  );
}

function PlanLimitsSetting({
  status,
  run,
}: {
  status: ClaudeStatus;
  run: (action: () => Promise<void>) => Promise<void>;
}) {
  const [preview, setPreview] = useState<string | null>(null);
  useEffect(() => {
    ipc.claudeStatusLinePreview().then(setPreview, () => setPreview(null));
  }, []);
  const { statusLine, statusLineTaken } = status.hooks;

  return (
    <Surface>
      <div className="settings__row">
        <span className="settings__row-text">
          <span className="settings__row-label">Plan limits</span>
          <span className="settings__row-description">
            Show your 5-hour and weekly limits in the notch. Claude Code shares them only with a status line,
            so this adds a small one that sends just those two numbers to Overline — and shows them in your
            terminal too. Only terminal (CLI) sessions update them; the desktop app doesn't run status lines.
          </span>
        </span>
        <Pill tone={statusLine ? "success" : "neutral"}>
          {statusLine ? "On" : statusLineTaken ? "Unavailable" : "Off"}
        </Pill>
      </div>
      {statusLineTaken ? (
        <p className="settings__note">
          You already have a status line in Claude Code, so Overline won't replace it.
        </p>
      ) : statusLine ? (
        <div className="settings__buttons">
          <ConfirmButton
            confirmLabel="Remove Overline's status line from Claude Code?"
            onConfirm={() => void run(ipc.claudeStatusLineDisable)}
          >
            Turn off
          </ConfirmButton>
        </div>
      ) : (
        <>
          {preview && (
            <details className="settings__details">
              <summary>Show exactly what will be added</summary>
              <pre className="settings__pre">{preview}</pre>
            </details>
          )}
          <div className="settings__buttons">
            <ConfirmButton
              confirmLabel="Add a status line to Claude Code settings?"
              onConfirm={() => void run(ipc.claudeStatusLineEnable)}
            >
              Show plan limits
            </ConfirmButton>
          </div>
        </>
      )}
    </Surface>
  );
}

export function AiSection() {
  const status = useClaudeStatus();
  const [preview, setPreview] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const installed = status?.hooks.installed ?? false;

  useEffect(() => {
    ipc.claudeHooksPreview().then(setPreview, () => setPreview(null));
  }, []);

  const run = async (action: () => Promise<void>) => {
    setError(null);
    try {
      await action();
    } catch (e) {
      setError(
        isIpcError(e) && e.kind === "invalid_input"
          ? `${e.message.replace(/^invalid input: /, "").replace(/^./, (c) => c.toUpperCase())}.`
          : "That didn't work. Check that the agent's settings file isn't open elsewhere, then try again.",
      );
    }
  };

  return (
    <Section title="AI" lead={pageLead("ai")}>
      <div id="setting-assistant">
        <AssistantSetting />
      </div>

      <h2 id="setting-coding-agents" className="settings__subhead">
        Coding agents
      </h2>
      <Surface>
        <div className="settings__row">
          <span className="settings__row-text">
            <span className="settings__row-label">Claude Code usage</span>
            <span className="settings__row-description">
              Estimated from Claude Code's transcripts on this PC. Only counts are read — never your prompts
              or code. No quota percentage is shown because none is recorded locally.
            </span>
          </span>
          {status && (
            <Pill tone={status.index.transcriptsFound ? "success" : "neutral"}>
              {status.index.transcriptsFound
                ? status.index.ready
                  ? "Up to date"
                  : "Reading…"
                : "No transcripts"}
            </Pill>
          )}
        </div>
      </Surface>

      <Surface>
        <div className="settings__row">
          <span className="settings__row-text">
            <span className="settings__row-label">Claude Code</span>
            <span className="settings__row-description">
              Show when Claude Code is working or waiting for you, and answer its permission requests from the
              notch. Adds hooks to Claude Code's settings that send events to Overline on this PC only. If
              Overline doesn't answer, Claude Code asks in the terminal as usual — nothing is ever allowed
              without your click.
            </span>
          </span>
          {installed ? (
            <Pill tone={status?.hooks.listening && !status.hooks.outdated ? "success" : "warning"}>
              {status?.hooks.outdated
                ? "Update needed"
                : status?.hooks.listening
                  ? "Connected"
                  : "Not listening"}
            </Pill>
          ) : (
            <Pill>Off</Pill>
          )}
        </div>

        {installed ? (
          <div className="settings__buttons">
            {status?.hooks.outdated && (
              <ConfirmButton
                confirmLabel="Update Overline's hooks in Claude Code settings?"
                onConfirm={() => void run(ipc.claudeConnect)}
              >
                Update hooks (adds approvals)
              </ConfirmButton>
            )}
            <ConfirmButton
              variant="danger"
              confirmLabel="Remove Overline's hooks from Claude Code?"
              onConfirm={() => void run(ipc.claudeDisconnect)}
            >
              Disconnect
            </ConfirmButton>
          </div>
        ) : (
          <>
            <p className="settings__lead">
              Connecting adds the entries below to <strong>~/.claude/settings.json</strong>. Your existing
              settings and hooks are kept, and a backup of the file is saved next to it first. If Overline
              isn't running, Claude Code simply carries on.
            </p>
            {preview && (
              <details className="settings__details">
                <summary>Show exactly what will be added</summary>
                <pre className="settings__pre">{preview}</pre>
              </details>
            )}
            <div className="settings__buttons">
              <ConfirmButton
                confirmLabel="Add hooks to Claude Code settings?"
                onConfirm={() => void run(ipc.claudeConnect)}
              >
                Connect Claude Code
              </ConfirmButton>
            </div>
          </>
        )}
        {error && <p className="settings__note settings__note--warning">{error}</p>}
        <p className="settings__note">New Claude Code sessions pick up the change; restart open ones.</p>
        {installed && (
          <div className="settings__buttons">
            <Button onClick={() => void ipc.claudeTestApproval()}>Send a test approval</Button>
            <span className="settings__note">
              Shows an example card in the notch. Nothing runs, whatever you click.
            </span>
          </div>
        )}
      </Surface>

      {installed && status && <PlanLimitsSetting status={status} run={run} />}

      <h2 id="setting-other-agents" className="settings__subhead">
        Other coding agents
      </h2>
      {status && <AgentsSetting agents={status.agents} run={run} />}
      {(installed || status?.agents.some((a) => a.connected)) && <RecentApprovals />}

      <Surface>
        <p className="settings__lead">Want to see the numbers? Open the AI agents module in the notch.</p>
        <div className="settings__buttons">
          <Button onClick={() => void ipc.openSettingsWindow("modules")}>Module settings</Button>
        </div>
      </Surface>
    </Section>
  );
}
