import { describe, expect, it } from "vitest";

import type { SpotifySnapshot } from "../../core/generated/SpotifySnapshot";
import { pinnedPlaylists, togglePinned } from "./spotifyStore";

const playlist = (id: string) => ({ id, name: id, trackCount: 1, owner: "me" });
const snapshot = (pinned: string[]): SpotifySnapshot => ({
  status: { state: "connected" },
  playlists: ["a", "b", "c", "d", "e", "f"].map(playlist),
  pinned,
  mock: false,
});

describe("pinnedPlaylists", () => {
  it("defaults to the first four in the library", () => {
    expect(pinnedPlaylists(snapshot([])).map((p) => p.id)).toEqual(["a", "b", "c", "d"]);
  });

  it("uses the user's picks in their order and skips deleted playlists", () => {
    expect(pinnedPlaylists(snapshot(["e", "gone", "a"])).map((p) => p.id)).toEqual(["e", "a"]);
  });

  it("is empty without a snapshot", () => {
    expect(pinnedPlaylists(null)).toEqual([]);
  });
});

describe("togglePinned", () => {
  it("adds in pick order, removes, and caps at four", () => {
    let pinned: string[] = [];
    for (const id of ["c", "a", "f", "b", "e"]) pinned = togglePinned(pinned, id);
    expect(pinned).toEqual(["c", "a", "f", "b"]);
    expect(togglePinned(pinned, "a")).toEqual(["c", "f", "b"]);
  });
});
