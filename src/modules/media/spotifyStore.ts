import { create } from "zustand";

import type { SpotifyPlaylist } from "../../core/generated/SpotifyPlaylist";
import type { SpotifySnapshot } from "../../core/generated/SpotifySnapshot";

export const MAX_PINNED = 4;

export interface SpotifyStoreState {
  snapshot: SpotifySnapshot | null;
  set: (snapshot: SpotifySnapshot) => void;
}

export const useSpotify = create<SpotifyStoreState>((set) => ({
  snapshot: null,
  set: (snapshot) => set({ snapshot }),
}));

export function isSpotifyConnected(snapshot: SpotifySnapshot | null): boolean {
  const state = snapshot?.status.state;
  return (
    state === "connected" ||
    (state === "error" && snapshot?.status.state === "error" && snapshot.status.reason === "network")
  );
}

/**
 * Playlists shown in the notch: the user's picks in their chosen order, skipping any that no
 * longer exist; the first four in their library until they pick.
 */
export function pinnedPlaylists(snapshot: SpotifySnapshot | null): SpotifyPlaylist[] {
  if (!snapshot) return [];
  if (snapshot.pinned.length === 0) return snapshot.playlists.slice(0, MAX_PINNED);
  const byId = new Map(snapshot.playlists.map((p) => [p.id, p]));
  return snapshot.pinned.flatMap((id) => byId.get(id) ?? []).slice(0, MAX_PINNED);
}

/** Toggle a playlist in the pinned list, keeping pick order and the four-playlist limit. */
export function togglePinned(pinned: string[], id: string): string[] {
  if (pinned.includes(id)) return pinned.filter((p) => p !== id);
  if (pinned.length >= MAX_PINNED) return pinned;
  return [...pinned, id];
}
