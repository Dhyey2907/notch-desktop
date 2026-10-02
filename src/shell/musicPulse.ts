// While music plays, the island glows in the album art's colors and pulses with the beat.
// The level arrives ~30 times a second, so it's written straight to CSS variables on the
// element instead of going through React state.
import { useEffect, type RefObject } from "react";

import { eventBus } from "../core/eventBus";
import { ipc } from "../core/ipc";
import { isLeadIsland } from "../core/window";
import { useReduceMotion } from "../design/useReduceMotion";
import { useMedia } from "../modules/media/mediaStore";
import { useSettings } from "../settings";

const BEAT_MS = 170;

export function useMusicPulse(island: RefObject<HTMLElement | null>) {
  const reduce = useReduceMotion();
  const playing = useMedia((s) => s.media?.status === "playing" && !s.media.mock);
  const artwork = useMedia((s) => s.media?.artworkPath);
  const off = useSettings((s) => s.settings?.music.hideReactive ?? false);
  const noBorder = useSettings((s) => s.settings?.music.hideBorder ?? false);
  const on = playing && !off;

  // Colors from the current album art.
  useEffect(() => {
    const el = island.current;
    if (!el) return;
    // The PC lights (OpenRGB) follow the music's color too.
    const lead = isLeadIsland();
    if (!on) {
      el.removeAttribute("data-music");
      if (lead) ipc.rgbMusicColor(null).catch(() => {});
      return;
    }
    el.setAttribute("data-music", "on");
    el.toggleAttribute("data-music-noborder", noBorder);
    if (!artwork) return;
    let alive = true;
    ipc
      .artworkPalette(artwork)
      .then((palette) => {
        if (!alive || !palette) return;
        el.style.setProperty("--music-primary", palette.primary);
        el.style.setProperty("--music-accent", palette.accent);
        if (lead) ipc.rgbMusicColor(palette.accent).catch(() => {});
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [island, on, artwork, noBorder]);

  // Loudness and beats.
  useEffect(() => {
    const el = island.current;
    if (!el || !on || reduce) {
      el?.style.setProperty("--music-level", "0");
      return;
    }
    let beatTimer: number | undefined;
    const offPulse = eventBus.on("MusicPulse", ({ event }) => {
      el.style.setProperty("--music-level", event.payload.level.toFixed(3));
      if (event.payload.beat) {
        el.classList.add("island--beat");
        window.clearTimeout(beatTimer);
        beatTimer = window.setTimeout(() => el.classList.remove("island--beat"), BEAT_MS);
      }
    });
    return () => {
      offPulse();
      window.clearTimeout(beatTimer);
      el.classList.remove("island--beat");
      el.style.setProperty("--music-level", "0");
    };
  }, [island, on, reduce]);
}
