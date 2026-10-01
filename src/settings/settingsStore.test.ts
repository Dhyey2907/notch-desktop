import { describe, expect, it, vi } from "vitest";

import { EventBus } from "../core/eventBus";
import type { Settings } from "../core/generated/Settings";
import { createSettingsStore, moduleSettings } from "./settingsStore";

const defaults: Settings = {
  firstRunCompleted: false,
  animation: "system",
  modules: {},
  calendar: { reminderMinutes: 5 },
  spotify: { pinnedPlaylists: [] },
  git: { repos: [] },
  display: { mode: "primary" },
  clipboard: { paused: false, blockedApps: [], noSuggestions: false },
  hud: { hideVolume: false, hideBrightness: false, hideBluetooth: false },
  music: { hideReactive: false, hideLyrics: false },
  game: { games: [], notGames: [], disabled: false },
  focus: {
    rain: 0,
    wind: 0,
    ocean: 0,
    fire: 0,
    brown: 0,
    pink: 0,
    master: 0,
    soundsOn: false,
    soundOwner: "",
    hidePet: false,
  },
  githubAlerts: { hideReviews: false, hideCiFailures: false, showCiPasses: false, hideMentions: false },
  appearance: {
    size: "normal",
    textSize: "normal",
    bubble: "pill",
    bubbleClock: false,
    theme: "black",
    opacity: 100,
    roundness: 100,
    accent: "",
    accentFromMusic: false,
    offsetX: 0,
    offsetY: 0,
  },
  behavior: {
    noHoverPeek: false,
    hoverSpeed: "normal",
    openOnHover: false,
    autoCloseSecs: 0,
    cardTime: "normal",
    quiet: { enabled: false, startMin: 1320, endMin: 420 },
    sound: "off",
    soundVolume: 0.6,
    fullscreen: "alerts",
  },
  shortcuts: { open: "Ctrl+Alt+N", ask: "Ctrl+Alt+A", clipboard: "Ctrl+Alt+V" },
  gestures: { disabled: false, sensitivity: "normal", noTrackSwipe: false },
  dock: { order: [], hidden: [] },
  home: { hiddenTiles: [], fahrenheit: false, clock24h: false },
};

function setup(initial: Settings = defaults) {
  let stored = initial;
  const backend = {
    get: vi.fn(async () => stored),
    update: vi.fn(async (next: Settings) => (stored = next)),
  };
  const events = new EventBus();
  return { store: createSettingsStore(backend, events), backend, events };
}

describe("settingsStore", () => {
  it("fills in module defaults for modules with no stored preferences", () => {
    expect(moduleSettings(defaults, "media")).toEqual({
      enabled: true,
      mayPeek: true,
      initFailures: 0,
      autoDisabled: false,
    });
  });

  it("patches one module without touching the others", async () => {
    const { store, backend } = setup({
      ...defaults,
      modules: { git: { enabled: false, mayPeek: true, initFailures: 0, autoDisabled: false } },
    });
    await store.getState().load();

    await store.getState().setModule("media", { mayPeek: false });

    const saved = backend.update.mock.calls[0][0];
    expect(saved.modules.git.enabled).toBe(false);
    expect(saved.modules.media).toMatchObject({ enabled: true, mayPeek: false });
  });

  it("adopts settings broadcast by another window", async () => {
    const { store, events } = setup();
    await store.getState().load();

    events.dispatch({
      id: "e1",
      v: 1,
      ts: "2026-09-28T00:00:00Z",
      source: "settings",
      event: { type: "SettingsChanged", payload: { ...defaults, animation: "reduced" } },
    });

    expect(store.getState().settings?.animation).toBe("reduced");
  });
});
