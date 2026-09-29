import { ListMusic } from "lucide-react";
import { useState } from "react";

import { ipc, isIpcError } from "../../core/ipc";
import { isSpotifyConnected, pinnedPlaylists, useSpotify } from "./spotifyStore";

/** Up to four pinned Spotify playlists; tapping one starts it on the active Spotify device. */
export function PlaylistTiles() {
  const snapshot = useSpotify((s) => s.snapshot);
  const [note, setNote] = useState<string | null>(null);
  const [pending, setPending] = useState<string | null>(null);

  if (!snapshot || snapshot.status.state === "notConfigured" || snapshot.status.state === "disconnected") {
    return (
      <button type="button" className="playlists__cta" onClick={() => void ipc.openSettingsWindow("media")}>
        <ListMusic size={14} aria-hidden /> Pin your Spotify playlists here
      </button>
    );
  }
  if (snapshot.status.state === "error" && snapshot.status.reason === "authExpired") {
    return (
      <button type="button" className="playlists__cta" onClick={() => void ipc.openSettingsWindow("media")}>
        Spotify needs to be reconnected
      </button>
    );
  }
  const playlists = pinnedPlaylists(snapshot);
  if (!isSpotifyConnected(snapshot) || playlists.length === 0) return null;

  const play = async (id: string, name: string) => {
    setPending(id);
    setNote(null);
    try {
      const outcome = await ipc.spotifyPlayPlaylist(id);
      if (outcome === "openedInApp") setNote(`Opened “${name}” in Spotify — press play there.`);
    } catch (error) {
      setNote(
        isIpcError(error) && error.kind === "invalid_input"
          ? `${error.message.replace(/^invalid input: /, "").replace(/^./, (c) => c.toUpperCase())}.`
          : "Spotify didn't respond. Try again.",
      );
    } finally {
      setPending(null);
    }
  };

  return (
    <div className="playlists">
      <div className="playlists__row" role="group" aria-label="Pinned Spotify playlists">
        {playlists.map((p) => (
          <button
            key={p.id}
            type="button"
            className={`playlists__tile${pending === p.id ? " playlists__tile--pending" : ""}`}
            title={`Play ${p.name}`}
            onClick={() => void play(p.id, p.name)}
          >
            {p.imageUrl ? (
              <img className="playlists__art" src={p.imageUrl} alt="" />
            ) : (
              <span className="playlists__art playlists__art--empty" aria-hidden>
                <ListMusic size={16} />
              </span>
            )}
            <span className="playlists__name">{p.name}</span>
          </button>
        ))}
      </div>
      {note && <p className="playlists__note">{note}</p>}
    </div>
  );
}
