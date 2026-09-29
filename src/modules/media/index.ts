// Media module (plan.md Phase 6): now-playing from the Windows media session.
import { Music2, Pause, Play, SkipBack, SkipForward } from "lucide-react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { lazy } from "react";

import type { MediaState } from "../../core/generated/MediaState";
import { ipc } from "../../core/ipc";
import type { ActivityAction } from "../../activity/activityStore";
import type { ModuleContext, NotchModule } from "../types";
import { MediaCompact, MediaPeek } from "./MediaIndicators";
import { shouldAnnounce, trackKey, useMedia } from "./mediaStore";
import { useSpotify } from "./spotifyStore";

const artworkUrl = (media: MediaState) => (media.artworkPath ? convertFileSrc(media.artworkPath) : undefined);

/** Previous / play-pause / next as inline icon controls that keep the activity open. */
function controls(media: MediaState): ActivityAction[] {
  const send = (command: Parameters<typeof ipc.mediaCommand>[0]) => () => ipc.mediaCommand(command);
  const playing = media.status === "playing";
  return [
    {
      id: "previous",
      label: "Previous track",
      icon: SkipBack,
      disabled: !media.capabilities.previous,
      keepOpen: true,
      run: send({ kind: "previous" }),
    },
    {
      id: "playPause",
      label: playing ? "Pause" : "Play",
      icon: playing ? Pause : Play,
      disabled: !media.capabilities.playPause,
      keepOpen: true,
      run: send({ kind: "playPause" }),
    },
    {
      id: "next",
      label: "Next track",
      icon: SkipForward,
      disabled: !media.capabilities.next,
      keepOpen: true,
      run: send({ kind: "next" }),
    },
  ];
}

function init({ events, activities }: ModuleContext) {
  // The track we last announced, so late-arriving artwork can be added to its peek.
  let announced: { id: string; key: string | null; hasArt: boolean } | null = null;

  const apply = (next: MediaState | null) => {
    const previous = useMedia.getState().media;
    useMedia.getState().set(next);
    if (next && announced && trackKey(next) === announced.key) {
      // Same track: keep the pop-up's controls (play/pause icon) and late artwork current.
      activities.update(announced.id, {
        actions: controls(next),
        ...(next.artworkPath && !announced.hasArt ? { imageUrl: artworkUrl(next) } : {}),
      });
      if (next.artworkPath) announced.hasArt = true;
    }
    if (next && shouldAnnounce(previous, next)) {
      // A brief, low-priority peek; a newer track replaces it in place.
      const id = activities.push({
        moduleId: "media",
        priority: "low",
        dedupKey: "media-track",
        // Long enough to reach the controls; hovering pauses the timer anyway.
        timeoutMs: 6000,
        countRepeats: false,
        actions: controls(next),
        title: next.title,
        body: [next.artist, next.sourceApp].filter(Boolean).join(" · "),
        imageUrl: artworkUrl(next),
      });
      announced = { id, key: trackKey(next), hasArt: Boolean(next.artworkPath) };
    }
  };

  const off = events.on("MediaStateChanged", ({ event }) => apply(event.payload.state));
  const offSpotify = events.on("SpotifyChanged", ({ event }) => useSpotify.getState().set(event.payload));
  ipc
    .spotifyGetSnapshot()
    .then((snapshot) => {
      if (!useSpotify.getState().snapshot) useSpotify.getState().set(snapshot);
    })
    .catch((error) => console.error("failed to read Spotify state", error));
  // Catch up on whatever was playing before this window started listening.
  ipc
    .mediaGetState()
    .then((state) => {
      if (useMedia.getState().media === undefined) useMedia.getState().set(state);
    })
    .catch((error) => console.error("failed to read media state", error));
  return () => {
    off();
    offSpotify();
  };
}

export const mediaModule: NotchModule = {
  id: "media",
  name: "Media",
  icon: Music2,
  order: 10,
  init,
  Compact: MediaCompact,
  Peek: MediaPeek,
  usePeekClaim: () => useMedia((s) => s.media?.status === "playing"),
  Expanded: lazy(() => import("./MediaExpanded")),
};
