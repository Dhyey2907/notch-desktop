import { useEffect, useState } from "react";

import { eventBus } from "../core/eventBus";
import type { ClaudeStatus } from "../core/generated/ClaudeStatus";
import { ipc, isIpcError } from "../core/ipc";
import { Button, ConfirmButton, Pill, Surface } from "../design/components";
import { Section } from "./Section";

function useClaudeStatus() {
  const [status, setStatus] = useState<ClaudeStatus | null>(null);
  useEffect(() => {
    ipc.claudeStatus().then(setStatus, () => setStatus(null));
    return eventBus.on("ClaudeStatusChanged", ({ event }) => setStatus(event.payload));
  }, []);
  return status;
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
          : "That didn't work. Check that Claude Code's settings file isn't open elsewhere, then try again.",
      );
    }
  };

  return (
    <Section title="AI">
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
            <span className="settings__row-label">Live activity</span>
            <span className="settings__row-description">
              Show when Claude Code is working or waiting for you. Adds hooks to Claude Code's settings that
              send events to Notch on this PC only.
            </span>
          </span>
          {installed ? (
            <Pill tone={status?.hooks.listening ? "success" : "warning"}>
              {status?.hooks.listening ? "Connected" : "Not listening"}
            </Pill>
          ) : (
            <Pill>Off</Pill>
          )}
        </div>

        {installed ? (
          <div className="settings__buttons">
            <ConfirmButton
              variant="danger"
              confirmLabel="Remove Notch's hooks from Claude Code?"
              onConfirm={() => void run(ipc.claudeDisconnect)}
            >
              Disconnect
            </ConfirmButton>
          </div>
        ) : (
          <>
            <p className="settings__lead">
              Connecting adds the entries below to <strong>~/.claude/settings.json</strong>. Your existing
              settings and hooks are kept, and a backup of the file is saved next to it first. If Notch isn't
              running, Claude Code simply carries on.
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
      </Surface>

      <Surface>
        <p className="settings__lead">Want to see the numbers? Open the Claude Code module in the notch.</p>
        <div className="settings__buttons">
          <Button onClick={() => void ipc.openSettingsWindow("modules")}>Module settings</Button>
        </div>
      </Surface>
    </Section>
  );
}
