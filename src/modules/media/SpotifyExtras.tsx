// Media, when Spotify is playing and connected: a heart to save the song to Liked Songs, and
// what's up next in the queue.
import { Heart, ListMusic } from "lucide-react";
import { useEffect, useState } from "react";

import type { SpotifyNow } from "../../core/generated/SpotifyNow";
import { ipc, isIpcError } from "../../core/ipc";
import { useSpotify } from "./spotifyStore";

export function SpotifyExtras({ title }: { title: string }) {
  const connected = useSpotify((s) => s.snapshot?.status.state === "connected");
  const [now, setNow] = useState<SpotifyNow | null>(null);
  const [showQueue, setShowQueue] = useState(false);
  const [note, setNote] = useState<string | null>(null);

  // Read again when the song changes.
  useEffect(() => {
    if (!connected) return;
    let live = true;
    ipc.spotifyNow().then(
      (n) => {
        if (!live) return;
        setNow(n);
        setNote(null);
      },
      (e) => live && setNote(isIpcError(e) ? e.message.replace(/^invalid input: /, "") : null),
    );
    return () => {
      live = false;
    };
  }, [title, connected]);

  if (!connected) return null;
  if (note) return <p className="media__note">{note}</p>;
  if (!now?.trackId) return null;

  const toggle = () => {
    const liked = !now.liked;
    setNow({ ...now, liked });
    void ipc.spotifyLike(now.trackId!, liked).catch((e) => {
      setNow({ ...now, liked: !liked });
      setNote(isIpcError(e) ? e.message.replace(/^invalid input: /, "") : "Spotify didn't take that");
    });
  };

  return (
    <div className="spotify-extras">
      <button
        type="button"
        className={`spotify-extras__heart${now.liked ? " spotify-extras__heart--on" : ""}`}
        aria-pressed={now.liked}
        aria-label={now.liked ? "Remove from Liked Songs" : "Save to Liked Songs"}
        title={now.liked ? "In your Liked Songs" : "Save to Liked Songs"}
        onClick={toggle}
      >
        <Heart size={15} aria-hidden fill={now.liked ? "currentColor" : "none"} />
      </button>
      {now.upNext.length > 0 && (
        <button
          type="button"
          className="spotify-extras__queue-btn"
          aria-expanded={showQueue}
          onClick={() => setShowQueue(!showQueue)}
        >
          <ListMusic size={14} aria-hidden /> Up next
        </button>
      )}
      {showQueue && (
        <ol className="spotify-extras__queue">
          {now.upNext.map((t, i) => (
            <li key={`${t}-${i}`}>{t}</li>
          ))}
        </ol>
      )}
    </div>
  );
}
