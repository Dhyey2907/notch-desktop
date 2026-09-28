// Development-only design gallery: every primitive rendered with clearly labeled sample
// content, for visual QA of the design system. Excluded from release builds.
import { Palette } from "lucide-react";
import { lazy } from "react";

import type { NotchModule } from "../types";
import { GalleryCompact, GalleryPeek } from "./GalleryIndicators";

export const galleryModule: NotchModule = {
  id: "gallery",
  name: "Design gallery",
  icon: Palette,
  order: 900,
  devOnly: true,
  Compact: GalleryCompact,
  Peek: GalleryPeek,
  // Marked pure so the bundler can drop this module from release builds.
  Expanded: /* @__PURE__ */ lazy(() => import("./GalleryExpanded")),
};
