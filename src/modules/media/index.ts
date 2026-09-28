// Media module (plan.md Phase 6): now-playing from the Windows media session.
import { Music2 } from "lucide-react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { lazy } from "react";

import type { MediaState } from "../../core/generated/MediaState";
import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { MediaCompact, MediaPeek } from "./MediaIndicators";
import { shouldAnnounce, trackKey, useMedia } from "./mediaStore";

const artworkUrl = (media: MediaState) => (media.artworkPath ? convertFileSrc(media.artworkPath) : undefined);

function init({ events, activities }: ModuleContext) {
  // The track we last announced, so late-arriving artwork can be added to its peek.
  let announced: { id: string; key: string | null; hasArt: boolean } | null = null;

  const apply = (next: MediaState | null) => {
    const previous = useMedia.getState().media;
    useMedia.getState().set(next);
    if (next && announced && !announced.hasArt && next.artworkPath && trackKey(next) === announced.key) {
      activities.update(announced.id, { imageUrl: artworkUrl(next) });
      announced.hasArt = true;
    }
    if (next && shouldAnnounce(previous, next)) {
      // A brief, low-priority peek; a newer track replaces it in place.
      const id = activities.push({
        moduleId: "media",
        priority: "low",
        dedupKey: "media-track",
        title: next.title,
        body: [next.artist, next.sourceApp].filter(Boolean).join(" · "),
        imageUrl: artworkUrl(next),
      });
      announced = { id, key: trackKey(next), hasArt: Boolean(next.artworkPath) };
    }
  };

  const off = events.on("MediaStateChanged", ({ event }) => apply(event.payload.state));
  // Catch up on whatever was playing before this window started listening.
  ipc
    .mediaGetState()
    .then((state) => {
      if (useMedia.getState().media === undefined) useMedia.getState().set(state);
    })
    .catch((error) => console.error("failed to read media state", error));
  return off;
}

export const mediaModule: NotchModule = {
  id: "media",
  name: "Media",
  icon: Music2,
  order: 10,
  init,
  Compact: MediaCompact,
  Peek: MediaPeek,
  Expanded: lazy(() => import("./MediaExpanded")),
};
