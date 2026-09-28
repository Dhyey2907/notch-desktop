import { create } from "zustand";

import type { MediaState } from "../../core/generated/MediaState";

export interface MediaStoreState {
  /** `undefined` until the first reading arrives; `null` when nothing is playing. */
  media: MediaState | null | undefined;
  set: (media: MediaState | null) => void;
}

export const useMedia = create<MediaStoreState>((set) => ({
  media: undefined,
  set: (media) => set({ media }),
}));

/** Identity of the current track, ignoring position and playback status. */
export function trackKey(media: MediaState | null | undefined): string | null {
  if (!media || !media.title) return null;
  return `${media.sourceApp}\u001f${media.title}\u001f${media.artist}`;
}

/** Announce a track only when a different one starts playing, not on pause/seek/startup. */
export function shouldAnnounce(previous: MediaState | null | undefined, next: MediaState | null): boolean {
  if (previous === undefined || !next || next.status !== "playing") return false;
  const key = trackKey(next);
  return key !== null && key !== trackKey(previous);
}

/**
 * Playback position at `now`, interpolated from the last sample while playing, so the UI
 * never needs to poll the backend (plan.md Rule 7).
 */
export function currentPositionMs(media: MediaState, now: number): number | null {
  if (media.positionMs == null) return null;
  const elapsed =
    media.status === "playing" ? Math.max(0, now - Date.parse(media.positionUpdatedAt)) * media.rate : 0;
  const position = media.positionMs + elapsed;
  return media.durationMs != null ? Math.min(position, media.durationMs) : position;
}

export function formatTime(ms: number): string {
  const total = Math.floor(ms / 1000);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = String(total % 60).padStart(2, "0");
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, "0")}:${seconds}` : `${minutes}:${seconds}`;
}
