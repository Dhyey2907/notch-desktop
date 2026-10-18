// Wallpaper: photos you save (Overline keeps its own copies) and the pictures in a folder you
// choose. Click one to make it the desktop wallpaper on one screen or all of them.
import { Wallpaper } from "lucide-react";
import { lazy } from "react";

import type { ModuleContext, NotchModule } from "../types";
import { useWallpaper } from "./wallpaperStore";

function init({ events }: ModuleContext) {
  return events.on("WallpaperChanged", ({ event }) => useWallpaper.getState().set(event.payload));
}

export const wallpaperModule: NotchModule = {
  id: "wallpaper",
  name: "Wallpaper",
  icon: Wallpaper,
  order: 56,
  init,
  Expanded: lazy(() => import("./WallpaperExpanded")),
};
