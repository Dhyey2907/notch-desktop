import type { MediaState } from "../../core/generated/MediaState";
import { ipc } from "../../core/ipc";
import { useCurrentLyric } from "./useCurrentLyric";

/** Two lines under the progress bar: the one being sung and the next. Click for the whole song
 *  in a floating tile. */
export function LyricsView({ media }: { media: MediaState }) {
  const lyric = useCurrentLyric(media);
  if (!lyric) return null;
  return (
    <button
      type="button"
      className="media__lyrics"
      aria-live="off"
      title="Open the full lyrics in a floating tile"
      onClick={() => void ipc.lyricsWindowOpen().catch(() => {})}
    >
      <p key={lyric.line} className="media__lyric media__lyric--now">
        {lyric.line}
      </p>
      {lyric.next && <p className="media__lyric">{lyric.next}</p>}
    </button>
  );
}
