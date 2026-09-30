import { useEffect, useState } from "react";

import { eventBus } from "../core/eventBus";
import type { CalendarSnapshot } from "../core/generated/CalendarSnapshot";
import { ipc, isIpcError } from "../core/ipc";
import {
  Button,
  ConfirmButton,
  MockBadge,
  Pill,
  SegmentedControl,
  Surface,
  TextField,
} from "../design/components";
import { useSettings } from ".";
import { Section } from "./Section";
import { SignInWith } from "./SignInWith";

const REMINDER_OPTIONS = [
  { value: "0", label: "Off" },
  { value: "1", label: "1 min" },
  { value: "5", label: "5 min" },
  { value: "10", label: "10 min" },
  { value: "15", label: "15 min" },
];

/** Calm, actionable wording for connect failures (design.md §23: no raw errors). */
function connectErrorMessage(error: unknown): string {
  if (isIpcError(error) && error.kind === "invalid_input") {
    const message = error.message.replace(/^invalid input: /, "");
    if (message.includes("timed out")) return "Sign-in timed out. Try again when you're ready.";
    if (message.includes("declined")) return "Sign-in was cancelled.";
    return `${message.charAt(0).toUpperCase()}${message.slice(1)}.`;
  }
  return "Couldn't finish connecting. Check the client ID and secret, then try again.";
}

function useCalendarSnapshot() {
  const [snapshot, setSnapshot] = useState<CalendarSnapshot | null>(null);
  useEffect(() => {
    ipc.calendarGetSnapshot().then(setSnapshot, () => setSnapshot(null));
    return eventBus.on("CalendarChanged", ({ event }) => setSnapshot(event.payload));
  }, []);
  return snapshot;
}

export function CalendarSection() {
  const snapshot = useCalendarSnapshot();
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  const savedClientId = settings?.calendar.clientId ?? "";
  const [clientId, setClientId] = useState(savedClientId);
  const [clientSecret, setClientSecret] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const state = snapshot?.status.state;
  const status = snapshot?.status;
  const needsReconnect = status?.state === "error" && status.reason === "authExpired";
  // A network/unknown error still counts as connected: saved events are shown and retries continue.
  const connected = state === "connected" || (status?.state === "error" && !needsReconnect);

  const connectBuiltin = async () => {
    setBusy(true);
    setError(null);
    try {
      await ipc.calendarConnectBuiltin();
    } catch (e) {
      setError(connectErrorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const connect = async () => {
    setBusy(true);
    setError(null);
    try {
      await ipc.calendarConnect(clientId, clientSecret);
      setClientSecret("");
    } catch (e) {
      setError(connectErrorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Section title="Calendar">
      <Surface>
        <div className="settings__row">
          <span className="settings__row-text">
            <span className="settings__row-label">Google Calendar {snapshot?.mock && <MockBadge />}</span>
            <span className="settings__row-description">
              {connected
                ? `Connected${snapshot?.account ? ` as ${snapshot.account}` : ""}`
                : state === "connecting"
                  ? "Waiting for you to finish signing in in your browser…"
                  : needsReconnect
                    ? "Access expired or was revoked. Reconnect to resume updates."
                    : "Not connected"}
            </span>
          </span>
          {connected && <Pill tone="success">Connected</Pill>}
          {needsReconnect && <Pill tone="warning">Needs attention</Pill>}
        </div>

        {connected ? (
          <>
            {snapshot?.lastSyncedAt && (
              <p className="settings__note">
                Last updated{" "}
                {new Date(snapshot.lastSyncedAt).toLocaleTimeString(undefined, {
                  hour: "numeric",
                  minute: "2-digit",
                })}
                {state === "error" && " · offline, showing saved events"}
              </p>
            )}
            <div className="settings__buttons">
              <Button onClick={() => void ipc.calendarSyncNow()}>Sync now</Button>
              <ConfirmButton
                variant="danger"
                confirmLabel="Disconnect and remove access?"
                onConfirm={() => void ipc.calendarDisconnect()}
              >
                Disconnect
              </ConfirmButton>
            </div>
          </>
        ) : (
          <SignInWith
            provider="google"
            busy={busy}
            reconnect={needsReconnect}
            error={error}
            onContinue={() => void connectBuiltin()}
          >
            <p className="settings__lead">
              Notch Desktop connects with your own Google Cloud OAuth client, so your calendar data goes only
              between your PC and Google. One-time setup:
            </p>
            <ol className="settings__steps">
              <li>
                Open <strong>console.cloud.google.com</strong> and create (or pick) a project.
              </li>
              <li>
                In <strong>APIs &amp; Services → Library</strong>, enable the{" "}
                <strong>Google Calendar API</strong>.
              </li>
              <li>
                In <strong>OAuth consent screen</strong>, choose <strong>External</strong> and add your Google
                account under <strong>Test users</strong>.
              </li>
              <li>
                In <strong>Credentials → Create credentials → OAuth client ID</strong>, choose application
                type <strong>Desktop app</strong>.
              </li>
              <li>Copy the client ID and client secret below, then connect.</li>
            </ol>
            <TextField
              label="Client ID"
              value={clientId}
              onChange={setClientId}
              placeholder="1234567890-abc.apps.googleusercontent.com"
              disabled={busy}
            />
            <TextField
              label="Client secret"
              type="password"
              value={clientSecret}
              onChange={setClientSecret}
              placeholder={savedClientId ? "Leave blank to use the saved secret" : "GOCSPX-…"}
              description="Stored in Windows Credential Manager, never in plain files."
              disabled={busy}
            />
            {error && <p className="settings__note settings__note--warning">{error}</p>}
            <div className="settings__buttons">
              <Button variant="primary" disabled={busy || !clientId.trim()} onClick={() => void connect()}>
                {busy ? "Waiting for sign-in…" : needsReconnect ? "Reconnect" : "Connect Google Calendar"}
              </Button>
            </div>
          </SignInWith>
        )}
        <p className="settings__note">
          Notch Desktop asks for read-only access to your calendar and your email address (to label the
          account). It never edits events.
        </p>
      </Surface>

      <Surface>
        <div className="settings__row">
          <span className="settings__row-text">
            <span className="settings__row-label">Meeting countdown</span>
            <span className="settings__row-description">How early the notch starts counting down.</span>
          </span>
          <SegmentedControl
            label="Meeting countdown"
            value={String(settings?.calendar.reminderMinutes ?? 5)}
            options={REMINDER_OPTIONS}
            onChange={(value) =>
              void update((s) => ({ ...s, calendar: { ...s.calendar, reminderMinutes: Number(value) } }))
            }
          />
        </div>
      </Surface>
    </Section>
  );
}
