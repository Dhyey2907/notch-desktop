import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useState, type KeyboardEvent, type MouseEvent } from "react";

import type { MediaCommand } from "../../core/generated/MediaCommand";
import type { MediaState } from "../../core/generated/MediaState";
import { ipc } from "../../core/ipc";
import { EmptyState, MediaControl, MockBadge, ModuleContainer, Pill } from "../../design/components";
import { currentPositionMs, formatTime, useMedia } from "./mediaStore";
import { PlaylistTiles } from "./PlaylistTiles";
import { SpotifyExtras } from "./SpotifyExtras";
import { LyricsView } from "./LyricsView";
import { brandBackground, brandOf, paletteBackground } from "./mediaTheme";
import { useArtworkPalette } from "./useArtworkPalette";
import "./media.css";

/** Re-render twice a second while playing, to move the progress bar. Local only — no IPC. */
function useNow(active: boolean) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!active) return;
    const id = window.setInterval(() => setNow(Date.now()), 500);
    return () => window.clearInterval(id);
  }, [active]);
  return now;
}

export default function MediaExpanded() {
  const media = useMedia((s) => s.media);
  const [failed, setFailed] = useState(false);

  const send = (command: MediaCommand) => {
    ipc.mediaCommand(command).then(
      () => setFailed(false),
      () => setFailed(true),
    );
  };

  if (media === undefined) return <ModuleContainer title="Media">{null}</ModuleContainer>;

  if (media === null) {
    return (
      <ModuleContainer title="Media">
        <div className="media__empty">
          <EmptyState
            title="Nothing playing"
            body="Play something in Spotify, a browser, or Media Player — or start one of your playlists."
          />
          <PlaylistTiles />
        </div>
      </ModuleContainer>
    );
  }

  return <NowPlaying media={media} failed={failed} send={send} />;
}

function NowPlaying({
  media,
  failed,
  send,
}: {
  media: MediaState;
  failed: boolean;
  send: (command: MediaCommand) => void;
}) {
  const playing = media.status === "playing";
  const now = useNow(playing);
  const position = currentPositionMs(media, now);
  const duration = media.durationMs ?? null;
  const artwork = media.artworkPath ? convertFileSrc(media.artworkPath) : null;
  const palette = useArtworkPalette(media.artworkPath);
  const brand = brandOf(media);
  const canSeek = media.capabilities.seek && duration !== null && duration > 0;

  const seekTo = (fraction: number) => {
    if (canSeek)
      send({ kind: "seek", positionMs: Math.round(Math.min(1, Math.max(0, fraction)) * duration) });
  };
  const onBarClick = (e: MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    seekTo((e.clientX - rect.left) / rect.width);
  };
  const onBarKey = (e: KeyboardEvent<HTMLDivElement>) => {
    if (!canSeek || position === null || (e.key !== "ArrowLeft" && e.key !== "ArrowRight")) return;
    e.preventDefault();
    seekTo((position + (e.key === "ArrowRight" ? 10_000 : -10_000)) / duration);
  };

  return (
    <div className="media">
      {/* The whole panel takes the playing thing's colors: the blurred album art and its own
          colors, or — without artwork — the service's brand gradient (Netflix red, Prime blue…). */}
      {artwork ? (
        <>
          <div className="media__backdrop" style={{ backgroundImage: `url("${artwork}")` }} aria-hidden />
          {palette && (
            <div
              className="media__tint"
              style={{ background: paletteBackground(palette.primary, palette.accent) }}
              aria-hidden
            />
          )}
        </>
      ) : (
        <div
          className="media__tint media__tint--brand"
          style={{ background: brandBackground(brand) }}
          aria-hidden
        />
      )}
      <div className="media__shade" aria-hidden />
      <ModuleContainer
        title={media.sourceApp}
        trailing={
          <>
            {media.mock && <MockBadge />}
            {!playing && <Pill>{media.status === "paused" ? "Paused" : "Stopped"}</Pill>}
          </>
        }
      >
        <div className="media__body">
          <div className="media__art" aria-hidden>
            {artwork ? <img src={artwork} alt="" /> : <span className="media__art-fallback">♪</span>}
          </div>
          <div className="media__main">
            {/* Some apps publish playback without track details; fall back to the app name. */}
            <p className="media__title" title={media.title || media.sourceApp}>
              {media.title || media.sourceApp}
            </p>
            <p className="media__meta">{[media.artist, media.album].filter(Boolean).join(" · ")}</p>

            {position !== null && duration !== null && duration > 0 && (
              <div className="media__timeline">
                <div
                  className={`media__bar${canSeek ? " media__bar--seekable" : ""}`}
                  role="slider"
                  aria-label="Playback position"
                  aria-valuemin={0}
                  aria-valuemax={Math.round(duration / 1000)}
                  aria-valuenow={Math.round(position / 1000)}
                  aria-valuetext={`${formatTime(position)} of ${formatTime(duration)}`}
                  aria-disabled={!canSeek}
                  tabIndex={canSeek ? 0 : -1}
                  onClick={onBarClick}
                  onKeyDown={onBarKey}
                >
                  <div className="media__bar-fill" style={{ width: `${(position / duration) * 100}%` }} />
                </div>
                <div className="media__times">
                  <span>{formatTime(position)}</span>
                  <span>-{formatTime(Math.max(0, duration - position))}</span>
                </div>
              </div>
            )}

            <LyricsView media={media} />

            <MediaControl
              playing={playing}
              canPrevious={media.capabilities.previous}
              canNext={media.capabilities.next}
              canPlayPause={media.capabilities.playPause}
              onPrevious={() => send({ kind: "previous" })}
              onPlayPause={() => send({ kind: "playPause" })}
              onNext={() => send({ kind: "next" })}
            />
            {failed && <p className="media__note">{media.sourceApp} didn't respond to that. Try again.</p>}
            {/spotify/i.test(media.sourceApp) && !media.mock && <SpotifyExtras title={media.title} />}
          </div>
        </div>
        <PlaylistTiles />
      </ModuleContainer>
    </div>
  );
}
