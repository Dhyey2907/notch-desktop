// While music plays, the island glows in the player's colors — the album art's, or the service's
// brand gradient when there's no art — and pulses with the beat.
// The level arrives ~30 times a second, so it's written straight to CSS variables on the
// element instead of going through React state.
import { useEffect, type RefObject } from "react";

import { eventBus } from "../core/eventBus";
import { ipc } from "../core/ipc";
import { isLeadIsland } from "../core/window";
import { useReduceMotion } from "../design/useReduceMotion";
import { useMedia } from "../modules/media/mediaStore";
import { brandOf } from "../modules/media/mediaTheme";
import { useSettings } from "../settings";

const BEAT_MS = 170;

export function useMusicPulse(island: RefObject<HTMLElement | null>) {
  const reduce = useReduceMotion();
  const playing = useMedia((s) => s.media?.status === "playing" && !s.media.mock);
  const artwork = useMedia((s) => s.media?.artworkPath);
  const brand = useMedia((s) => (s.media ? brandOf(s.media) : null));
  const off = useSettings((s) => s.settings?.music.hideReactive ?? false);
  const noBorder = useSettings((s) => s.settings?.music.hideBorder ?? false);
  const on = playing && !off;

  // The player's colors: the album art's, or the service's brand.
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
    const paint = (accent: string, primary: string) => {
      el.style.setProperty("--music-accent", accent);
      el.style.setProperty("--music-primary", primary);
      if (lead) ipc.rgbMusicColor(accent).catch(() => {});
    };
    // No artwork (Netflix in a browser…): the same brand gradient as the Media panel.
    if (!artwork) {
      if (brand) paint(brand.colors[0], brand.colors[1]);
      else {
        el.style.removeProperty("--music-accent");
        el.style.removeProperty("--music-primary");
      }
      return;
    }
    let alive = true;
    ipc
      .artworkPalette(artwork)
      .then((palette) => {
        if (alive && palette) paint(palette.accent, palette.primary);
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [island, on, artwork, brand, noBorder]);

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
