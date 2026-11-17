// The floating lyrics tile (#lyrics), opened by clicking the lyrics in Media: the whole song,
// the line being sung lit and kept in the middle, over the album's own colors. Drag the top bar
// to move it, the edges to resize; it stays on top and comes back where you left it. Click a
// line to jump there when the player can seek.
import { convertFileSrc } from "@tauri-apps/api/core";
import { X } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { eventBus } from "../core/eventBus";
import type { Lyrics } from "../core/generated/Lyrics";
import type { MediaState } from "../core/generated/MediaState";
import { ipc } from "../core/ipc";
import { currentLineIndex } from "../modules/media/lyrics";
import { brandBackground, brandOf, paletteBackground } from "../modules/media/mediaTheme";
import { currentPositionMs } from "../modules/media/mediaStore";
import { useArtworkPalette } from "../modules/media/useArtworkPalette";
import "./lyricsTile.css";

/** Lines move a few times a second while playing. */
const TICK_MS = 250;
/** After you scroll yourself, the tile waits this long before following the song again. */
const HANDS_OFF_MS = 4000;

export function LyricsTileApp() {
  const [media, setMedia] = useState<MediaState | null>(null);
  const [lyrics, setLyrics] = useState<Lyrics | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const listRef = useRef<HTMLOListElement>(null);
  const touchedAt = useRef(0);

  useEffect(() => {
    ipc.mediaGetState().then(setMedia, () => {});
    ipc.lyricsCurrent().then(setLyrics, () => {});
    const offMedia = eventBus.on("MediaStateChanged", ({ event }) => setMedia(event.payload.state));
    const offLyrics = eventBus.on("LyricsChanged", ({ event }) => setLyrics(event.payload));
    return () => {
      offMedia();
      offLyrics();
    };
  }, []);

  const playing = media?.status === "playing";
  useEffect(() => {
    if (!playing) return;
    const id = window.setInterval(() => setNow(Date.now()), TICK_MS);
    return () => window.clearInterval(id);
  }, [playing]);

  // Only lyrics for the song playing now (a late answer may belong to the last one).
  const lines =
    media && lyrics && lyrics.track === `${media.title.trim()} — ${media.artist.trim()}` ? lyrics.lines : [];
  const position = media ? currentPositionMs(media, now) : null;
  const current = lines.length && position !== null ? currentLineIndex(lines, position) : -1;

  // Keep the line being sung in the middle, unless you're scrolling yourself. Only the list
  // scrolls: scrollIntoView would also scroll the window and push the top bar out of sight.
  useEffect(() => {
    const list = listRef.current;
    if (!list || current < 0 || Date.now() - touchedAt.current < HANDS_OFF_MS) return;
    const el = list.children[current] as HTMLElement | undefined;
    if (!el) return;
    list.scrollTo({ top: el.offsetTop - list.clientHeight / 2 + el.offsetHeight / 2, behavior: "smooth" });
  }, [current]);

  const artwork = media?.artworkPath ? convertFileSrc(media.artworkPath) : null;
  const palette = useArtworkPalette(media?.artworkPath);
  const canSeek = Boolean(media?.capabilities.seek);

  let body;
  if (!media || media.status === "stopped") {
    body = <p className="lyrics-tile__empty">Nothing is playing.</p>;
  } else if (lines.length === 0) {
    body = <p className="lyrics-tile__empty">No synced lyrics for this song.</p>;
  } else {
    body = (
      <ol
        ref={listRef}
        className="lyrics-tile__lines"
        onWheel={() => (touchedAt.current = Date.now())}
        onPointerDown={() => (touchedAt.current = Date.now())}
      >
        {lines.map((line, i) => (
          <li
            key={`${line.atMs}-${i}`}
            className={`lyrics-tile__line${i === current ? " lyrics-tile__line--now" : ""}${i < current ? " lyrics-tile__line--past" : ""}`}
          >
            {canSeek ? (
              <button
                type="button"
                title="Play from here"
                onClick={() => {
                  touchedAt.current = 0;
                  void ipc.mediaCommand({ kind: "seek", positionMs: line.atMs }).catch(() => {});
                }}
              >
                {line.text || "♪"}
              </button>
            ) : (
              <span>{line.text || "♪"}</span>
            )}
          </li>
        ))}
      </ol>
    );
  }

  return (
    <div className="lyrics-tile">
      {artwork ? (
        <>
          <div
            className="lyrics-tile__backdrop"
            style={{ backgroundImage: `url("${artwork}")` }}
            aria-hidden
          />
          {palette && (
            <div
              className="lyrics-tile__tint"
              style={{ background: paletteBackground(palette.primary, palette.accent) }}
              aria-hidden
            />
          )}
        </>
      ) : (
        <div
          className="lyrics-tile__tint lyrics-tile__tint--brand"
          style={{ background: brandBackground(media ? brandOf(media) : null) }}
          aria-hidden
        />
      )}
      <div className="lyrics-tile__shade" aria-hidden />

      <header className="lyrics-tile__bar" data-tauri-drag-region>
        {artwork && <img className="lyrics-tile__art" src={artwork} alt="" data-tauri-drag-region />}
        <div className="lyrics-tile__song" data-tauri-drag-region>
          <p className="lyrics-tile__title" data-tauri-drag-region>
            {media?.title || "Lyrics"}
          </p>
          {media?.artist && (
            <p className="lyrics-tile__artist" data-tauri-drag-region>
              {media.artist}
            </p>
          )}
        </div>
        <button
          type="button"
          className="lyrics-tile__close"
          aria-label="Close lyrics"
          title="Close"
          onClick={() => void ipc.lyricsWindowClose()}
        >
          <X size={14} aria-hidden />
        </button>
      </header>
      <div className="lyrics-tile__body">{body}</div>
    </div>
  );
}
