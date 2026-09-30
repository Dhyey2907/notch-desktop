// Whether a fullscreen app (video, game, F11 browser) is in front. While it is, the island
// hides, pops in for notifications, and hides again; it comes back when fullscreen ends.
import { create } from "zustand";

import { eventBus } from "../core/eventBus";
import { ipc } from "../core/ipc";
import { windowLabel } from "../core/window";

export const useFullscreen = create<{ active: boolean }>(() => ({ active: false }));

/** Start tracking; returns a cleanup function. */
export function trackFullscreen(): () => void {
  let cancelled = false;
  // Each screen has its own island; only this screen's fullscreen state matters here.
  const off = eventBus.on("FullscreenChanged", ({ event }) => {
    if (event.payload.label === windowLabel()) useFullscreen.setState({ active: event.payload.active });
  });
  ipc.shellIsFullscreen().then(
    (active) => !cancelled && useFullscreen.setState({ active }),
    () => {},
  );
  return () => {
    cancelled = true;
    off();
  };
}
