import { useEffect, useState } from "react";

import type { Palette } from "../../core/generated/Palette";
import { ipc } from "../../core/ipc";

/** The two main colors of the album art at `path` (null until read, or without artwork). */
export function useArtworkPalette(path: string | undefined): Palette | null {
  const [palette, setPalette] = useState<{ path: string; palette: Palette | null } | null>(null);
  useEffect(() => {
    if (!path) return;
    let alive = true;
    ipc.artworkPalette(path).then(
      (p) => alive && setPalette({ path, palette: p }),
      () => alive && setPalette({ path, palette: null }),
    );
    return () => {
      alive = false;
    };
  }, [path]);
  return path && palette?.path === path ? palette.palette : null;
}
