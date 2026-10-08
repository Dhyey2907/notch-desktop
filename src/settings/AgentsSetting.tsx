// Settings → AI: connect Codex and Gemini CLI the same way as Claude Code — hooks in their own
// config that send events to Notch on this PC, shown in full before anything is written.
import { useState } from "react";

import type { AgentLink } from "../core/generated/AgentLink";
import { ipc } from "../core/ipc";
import { ConfirmButton, Pill, Surface } from "../design/components";

const ABOUT: Record<string, { file: string; does: string; note: string }> = {
  codex: {
    file: "~/.codex/hooks.json",
    does: "Show when Codex is working or waiting for you, and allow or deny its approval requests from the notch — the same as Claude Code. Works with the Codex app and the codex command.",
    note: "Codex runs new hooks only after you trust them once: open Codex, type /hooks and trust Overline's.",
  },
  gemini: {
    file: "~/.gemini/settings.json",
    does: "Show when Gemini CLI is working, alert you when it needs your OK, and tell you when it's done. Approve tool calls in Gemini's terminal as usual.",
    note: "New Gemini CLI sessions pick up the change; restart open ones.",
  },
};

export function AgentsSetting({
  agents,
  run,
}: {
  agents: AgentLink[];
  run: (action: () => Promise<void>) => Promise<void>;
}) {
  return (
    <>
      {agents.map((link) => (
        <AgentCard key={link.agent} link={link} run={run} />
      ))}
    </>
  );
}

function AgentCard({ link, run }: { link: AgentLink; run: (action: () => Promise<void>) => Promise<void> }) {
  const [preview, setPreview] = useState<string | null>(null);
  const about = ABOUT[link.agent];
  if (!about) return null;
  return (
    <Surface>
      <div className="settings__row">
        <span className="settings__row-text">
          <span className="settings__row-label">{link.name}</span>
          <span className="settings__row-description">{about.does}</span>
        </span>
        <Pill tone={link.connected ? "success" : "neutral"}>
          {link.connected ? "Connected" : link.detected ? "Off" : "Not installed"}
        </Pill>
      </div>
      {link.connected ? (
        <div className="settings__buttons">
          <ConfirmButton
            variant="danger"
            confirmLabel={`Remove Overline's hooks from ${link.name}?`}
            onConfirm={() => void run(() => ipc.agentDisconnect(link.agent))}
          >
            Disconnect
          </ConfirmButton>
        </div>
      ) : (
        <>
          <p className="settings__lead">
            Connecting adds hooks to <strong>{about.file}</strong>. Your own settings are kept, and a backup
            is saved next to it first. If Overline isn't running, {link.name} simply carries on.
            {!link.detected && ` ${link.name} wasn't found on this PC, but you can connect it now for later.`}
          </p>
          <details
            className="settings__details"
            onToggle={(e) => {
              if (e.currentTarget.open && preview === null)
                ipc.agentPreview(link.agent).then(setPreview, () => setPreview(""));
            }}
          >
            <summary>Show exactly what will be added</summary>
            {preview && <pre className="settings__pre">{preview}</pre>}
          </details>
          <div className="settings__buttons">
            <ConfirmButton
              confirmLabel={`Add hooks to ${link.name}?`}
              onConfirm={() => void run(() => ipc.agentConnect(link.agent))}
            >
              Connect {link.name}
            </ConfirmButton>
          </div>
        </>
      )}
      <p className="settings__note">{about.note}</p>
    </Surface>
  );
}
