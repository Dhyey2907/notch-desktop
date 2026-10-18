// "Accent from wallpaper" (Settings → Look): the notch takes its accent colour from the main
// screen's wallpaper, and follows it when the wallpaper changes.
import { useEffect } from "react";
import { create } from "zustand";

import { eventBus } from "../core/eventBus";
import { ipc } from "../core/ipc";
import { useSettings } from "../settings";

export const useWallpaperColor = create<{ accent: string | null }>(() => ({ accent: null }));

export function useWallpaperAccent() {
  const on = useSettings((s) => s.settings?.appearance.accentFromWallpaper ?? false);
  useEffect(() => {
    if (!on) {
      useWallpaperColor.setState({ accent: null });
      return;
    }
    let live = true;
    const read = () =>
      ipc.wallpaperAccent().then(
        (palette) => live && useWallpaperColor.setState({ accent: palette?.accent ?? null }),
        () => {},
      );
    void read();
    // Windows reports a new wallpaper a moment after it changes.
    const off = eventBus.on("WallpaperChanged", () => void read());
    const id = window.setInterval(() => void read(), 5 * 60_000);
    return () => {
      live = false;
      off();
      window.clearInterval(id);
    };
  }, [on]);
}
