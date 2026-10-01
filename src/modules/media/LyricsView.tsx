import type { MediaState } from "../../core/generated/MediaState";
import { useCurrentLyric } from "./useCurrentLyric";

/** Two lines under the progress bar: the one being sung and the next. */
export function LyricsView({ media }: { media: MediaState }) {
  const lyric = useCurrentLyric(media);
  if (!lyric) return null;
  return (
    <div className="media__lyrics" aria-live="off">
      <p key={lyric.line} className="media__lyric media__lyric--now">
        {lyric.line}
      </p>
      {lyric.next && <p className="media__lyric">{lyric.next}</p>}
    </div>
  );
}
