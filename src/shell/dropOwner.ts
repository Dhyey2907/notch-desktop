// A module that takes dropped files itself while it's open (the Wallpaper panel keeps dropped
// pictures). The shelf and the "open Clipboard on drag" behaviour stay out of those drops.
import { create } from "zustand";

import { useIsland } from "./useIsland";

export const useDropOwner = create<{ owner: string | null }>(() => ({ owner: null }));

/** Claim drops while this module's panel is showing; returns the release. */
export function claimDrops(module: string): () => void {
  useDropOwner.setState({ owner: module });
  return () => {
    if (useDropOwner.getState().owner === module) useDropOwner.setState({ owner: null });
  };
}

/** The open panel takes this drop itself. */
export function dropTakenByOpenModule(): boolean {
  return useIsland.getState().mode === "expanded" && useDropOwner.getState().owner !== null;
}
