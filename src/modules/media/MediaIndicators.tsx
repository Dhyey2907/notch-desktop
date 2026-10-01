import { convertFileSrc } from "@tauri-apps/api/core";
import { Music2, Pause, Play, SkipBack, SkipForward } from "lucide-react";

import type { MediaCommand } from "../../core/generated/MediaCommand";
import { ipc } from "../../core/ipc";
import { IconButton, MockBadge } from "../../design/components";
import { useMedia } from "./mediaStore";
import { useCurrentLyric } from "./useCurrentLyric";
import "./media.css";

/** Collapsed indicator: a small note while something is playing, nothing otherwise. */
export function MediaCompact() {
  const playing = useMedia((s) => s.media?.status === "playing");
  if (!playing) return null;
  return <Music2 className="media-compact" size={13} role="img" aria-label="Music playing" />;
}

export function MediaPeek() {
  const media = useMedia((s) => s.media);
  const lyric = useCurrentLyric(media);
  if (!media) {
    return (
      <div className="module-peek">
        <span className="module-peek__title">Nothing playing</span>
        <span className="module-peek__meta">Click to open</span>
      </div>
    );
  }
  const playing = media.status === "playing";
  const send = (command: MediaCommand) => void ipc.mediaCommand(command).catch(() => {});
  return (
    <div className="media-peek">
      {media.artworkPath ? (
        <img className="media-peek__art" src={convertFileSrc(media.artworkPath)} alt="" />
      ) : (
        <Music2 className="media-peek__art media-peek__art--empty" size={18} aria-hidden />
      )}
      <div className="media-peek__text">
        <span className="module-peek__title">
          {media.title || media.sourceApp} {media.mock && <MockBadge />}
        </span>
        <span className="module-peek__meta">
          {lyric && playing ? lyric.line : [media.artist, media.sourceApp].filter(Boolean).join(" · ")}
        </span>
      </div>
      {/* Above the island's click-to-open layer, so these act on playback instead. */}
      <div className="media-peek__controls" role="group" aria-label="Playback controls">
        <IconButton
          icon={SkipBack}
          label="Previous track"
          disabled={!media.capabilities.previous}
          onClick={() => send({ kind: "previous" })}
        />
        <IconButton
          icon={playing ? Pause : Play}
          label={playing ? "Pause" : "Play"}
          disabled={!media.capabilities.playPause}
          onClick={() => send({ kind: "playPause" })}
        />
        <IconButton
          icon={SkipForward}
          label="Next track"
          disabled={!media.capabilities.next}
          onClick={() => send({ kind: "next" })}
        />
      </div>
    </div>
  );
}
