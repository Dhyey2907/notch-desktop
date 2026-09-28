import { Music2 } from "lucide-react";

import { MockBadge } from "../../design/components";
import { useMedia } from "./mediaStore";
import "./media.css";

/** Collapsed indicator: a small note while something is playing, nothing otherwise. */
export function MediaCompact() {
  const playing = useMedia((s) => s.media?.status === "playing");
  if (!playing) return null;
  return <Music2 className="media-compact" size={13} role="img" aria-label="Music playing" />;
}

export function MediaPeek() {
  const media = useMedia((s) => s.media);
  if (!media) {
    return (
      <div className="module-peek">
        <span className="module-peek__title">Nothing playing</span>
        <span className="module-peek__meta">Click to open</span>
      </div>
    );
  }
  return (
    <div className="module-peek">
      <span className="module-peek__title">
        {media.title || media.sourceApp} {media.mock && <MockBadge />}
      </span>
      <span className="module-peek__meta">{[media.artist, media.sourceApp].filter(Boolean).join(" · ")}</span>
    </div>
  );
}
