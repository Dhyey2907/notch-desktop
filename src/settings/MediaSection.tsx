import { Check, ListMusic } from "lucide-react";
import { useEffect, useState } from "react";

import { eventBus } from "../core/eventBus";
import type { SpotifySnapshot } from "../core/generated/SpotifySnapshot";
import { ipc, isIpcError } from "../core/ipc";
import { Button, ConfirmButton, MockBadge, Pill, Surface, TextField } from "../design/components";
import { MAX_PINNED, pinnedPlaylists, togglePinned } from "../modules/media/spotifyStore";
import { useSettings } from ".";
import { Section } from "./Section";

const REDIRECT_URI = "http://127.0.0.1:43821/callback";

function useSpotifySnapshot() {
  const [snapshot, setSnapshot] = useState<SpotifySnapshot | null>(null);
  useEffect(() => {
    ipc.spotifyGetSnapshot().then(setSnapshot, () => setSnapshot(null));
    return eventBus.on("SpotifyChanged", ({ event }) => setSnapshot(event.payload));
  }, []);
  return snapshot;
}

function friendlyError(error: unknown, fallback: string): string {
  if (isIpcError(error) && error.kind === "invalid_input") {
    const message = error.message.replace(/^invalid input: /, "");
    if (message.includes("timed out")) return "Sign-in timed out. Try again when you're ready.";
    if (message.includes("declined")) return "Sign-in was cancelled.";
    return `${message.charAt(0).toUpperCase()}${message.slice(1)}.`;
  }
  return fallback;
}

export function MediaSection() {
  const snapshot = useSpotifySnapshot();
  const savedClientId = useSettings((s) => s.settings?.spotify.clientId ?? "");
  const [clientId, setClientId] = useState(savedClientId);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const status = snapshot?.status;
  const needsReconnect = status?.state === "error" && status.reason === "authExpired";
  const connected = status?.state === "connected" || (status?.state === "error" && !needsReconnect);

  const connect = async () => {
    setBusy(true);
    setError(null);
    try {
      await ipc.spotifyConnect(clientId);
    } catch (e) {
      setError(
        friendlyError(e, "Couldn't finish connecting. Check the client ID and redirect URI, then try again."),
      );
    } finally {
      setBusy(false);
    }
  };

  return (
    <Section title="Media">
      <Surface>
        <div className="settings__row">
          <span className="settings__row-text">
            <span className="settings__row-label">Spotify playlists {snapshot?.mock && <MockBadge />}</span>
            <span className="settings__row-description">
              {connected
                ? `Connected${snapshot?.account ? ` as ${snapshot.account}` : ""}`
                : status?.state === "connecting"
                  ? "Waiting for you to finish signing in in your browser…"
                  : needsReconnect
                    ? "Access expired or was revoked. Reconnect to keep your playlists."
                    : "Pin up to four playlists to start them from the notch."}
            </span>
          </span>
          {connected && snapshot?.premium === true && <Pill tone="success">Premium</Pill>}
          {connected && snapshot?.premium === false && <Pill tone="warning">Free · opens in Spotify</Pill>}
          {needsReconnect && <Pill tone="warning">Needs attention</Pill>}
        </div>

        {connected && snapshot ? (
          <>
            <div className="settings__buttons">
              <Button onClick={() => void ipc.spotifyRefreshPlaylists().catch(() => {})}>
                Refresh playlists
              </Button>
              <ConfirmButton
                variant="danger"
                confirmLabel="Disconnect Spotify?"
                onConfirm={() => void ipc.spotifyDisconnect()}
              >
                Disconnect
              </ConfirmButton>
            </div>
            <p className="settings__note">
              Spotify doesn't offer a way for apps to sign you out remotely; you can also remove access at
              spotify.com/account/apps.
            </p>
          </>
        ) : (
          <>
            <p className="settings__lead">
              Uses your own Spotify developer app (free, takes a couple of minutes). One-time setup:
            </p>
            <ol className="settings__steps">
              <li>
                Open <strong>developer.spotify.com/dashboard</strong> and choose <strong>Create app</strong>.
              </li>
              <li>
                Give it any name and description. Under <strong>Redirect URIs</strong>, add{" "}
                <strong className="settings__code">{REDIRECT_URI}</strong>.
              </li>
              <li>
                Under <strong>Which API/SDKs</strong>, tick <strong>Web API</strong>, accept the terms, and
                save.
              </li>
              <li>
                Open the app's <strong>Settings</strong>, copy the <strong>Client ID</strong>, paste it below,
                and connect.
              </li>
            </ol>
            <TextField
              label="Client ID"
              value={clientId}
              onChange={setClientId}
              placeholder="32 characters, e.g. 0123456789abcdef0123456789abcdef"
              description="No client secret is needed — sign-in uses PKCE."
              disabled={busy}
            />
            {error && <p className="settings__note settings__note--warning">{error}</p>}
            <div className="settings__buttons">
              <Button variant="primary" disabled={busy || !clientId.trim()} onClick={() => void connect()}>
                {busy ? "Waiting for sign-in…" : needsReconnect ? "Reconnect" : "Connect Spotify"}
              </Button>
            </div>
          </>
        )}
      </Surface>

      {connected && snapshot && <PlaylistPicker snapshot={snapshot} />}
    </Section>
  );
}

function PlaylistPicker({ snapshot }: { snapshot: SpotifySnapshot }) {
  const [error, setError] = useState<string | null>(null);
  // Until the user picks, the notch shows the first four; start the picker from that.
  const pinned = snapshot.pinned.length > 0 ? snapshot.pinned : pinnedPlaylists(snapshot).map((p) => p.id);
  const explicit = snapshot.pinned.length > 0;

  const toggle = (id: string) => {
    const next = togglePinned(pinned, id);
    if (next === pinned) return; // already at four
    ipc.spotifySetPinned(next).then(
      () => setError(null),
      () => setError("Couldn't save your picks. Try again."),
    );
  };

  return (
    <Surface>
      <div className="settings__row">
        <span className="settings__row-text">
          <span className="settings__row-label">Playlists in the notch</span>
          <span className="settings__row-description">
            {explicit
              ? `${pinned.length} of ${MAX_PINNED} picked, in the order shown in the notch.`
              : `Showing your first ${MAX_PINNED}. Pick up to ${MAX_PINNED} to choose your own.`}
          </span>
        </span>
      </div>
      {error && <p className="settings__note settings__note--warning">{error}</p>}
      {snapshot.playlists.length === 0 ? (
        <p className="settings__note">No playlists found in your Spotify library.</p>
      ) : (
        <ul className="picker" aria-label="Your Spotify playlists">
          {snapshot.playlists.map((p) => {
            const index = pinned.indexOf(p.id);
            const selected = index >= 0;
            const full = !selected && pinned.length >= MAX_PINNED;
            return (
              <li key={p.id}>
                <button
                  type="button"
                  className={`picker__item${selected ? " picker__item--selected" : ""}`}
                  aria-pressed={selected}
                  disabled={full}
                  onClick={() => toggle(p.id)}
                >
                  {p.imageUrl ? (
                    <img className="picker__art" src={p.imageUrl} alt="" />
                  ) : (
                    <span className="picker__art picker__art--empty" aria-hidden>
                      <ListMusic size={14} />
                    </span>
                  )}
                  <span className="picker__text">
                    <span className="picker__name">{p.name}</span>
                    <span className="picker__meta">
                      {p.trackCount} tracks{p.owner ? ` · ${p.owner}` : ""}
                    </span>
                  </span>
                  <span className="picker__mark" aria-hidden>
                    {selected ? explicit ? index + 1 : <Check size={12} /> : null}
                  </span>
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </Surface>
  );
}
