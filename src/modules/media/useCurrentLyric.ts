import { useEffect, useState } from "react";

import type { MediaState } from "../../core/generated/MediaState";
import { currentLineIndex, useLyrics } from "./lyrics";
import { currentPositionMs, trackKey } from "./mediaStore";

/** Lyrics move a few times a second while playing. */
const TICK_MS = 250;

function useTick(on: boolean): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!on) return;
    const id = window.setInterval(() => setNow(Date.now()), TICK_MS);
    return () => window.clearInterval(id);
  }, [on]);
  return now;
}

/** The lyric line at the current position for this track, or null. */
export function useCurrentLyric(
  media: MediaState | null | undefined,
): { line: string; next?: string } | null {
  const lyrics = useLyrics((s) => s.lyrics);
  const playing = media?.status === "playing";
  const now = useTick(Boolean(playing && lyrics?.lines.length));
  if (!media || !lyrics || lyrics.lines.length === 0 || !trackKey(media)) return null;
  if (lyrics.track !== `${media.title.trim()} — ${media.artist.trim()}`) return null;
  const position = currentPositionMs(media, now);
  if (position === null) return null;
  const i = currentLineIndex(lyrics.lines, position);
  if (i < 0) return null;
  return { line: lyrics.lines[i].text || "♪", next: lyrics.lines[i + 1]?.text };
}
