import { describe, expect, it, vi } from "vitest";

import { EventBus } from "../core/eventBus";
import type { Settings } from "../core/generated/Settings";
import { createSettingsStore, moduleSettings } from "./settingsStore";

const defaults: Settings = {
  firstRunCompleted: false,
  animation: "system",
  modules: {},
  calendar: { reminderMinutes: 5, gmail: false, classroom: false, edit: false, grantedScopes: [] },
  spotify: { pinnedPlaylists: [] },
  git: { repos: [] },
  display: { mode: "primary" },
  clipboard: {
    paused: false,
    blockedApps: [],
    noSuggestions: false,
    noLinkCleaning: false,
    noSnippets: false,
    shelfKeep: "",
    noPhoneCodes: false,
    noScreenshots: false,
  },
  hud: {
    hideVolume: false,
    hideBrightness: false,
    hideBluetooth: false,
    hideCaps: false,
    hideLanguage: false,
    hideInternet: false,
    hideDeviceLow: false,
    hideCharging: false,
    noChargeAlert: false,
    chargeAlertAt: 0,
    dataCapGb: 0,
  },
  music: { hideReactive: false, hideLyrics: false, hideBorder: false },
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
    accentFromWallpaper: false,
    offsetX: 0,
    offsetY: 0,
    edge: "top",
    offsetSide: 160,
    fadeWhileTyping: false,
    lockscreenInfo: false,
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
  shortcuts: {
    open: "Ctrl+Alt+N",
    ask: "Ctrl+Alt+A",
    clipboard: "Ctrl+Alt+V",
    grab: "Ctrl+Alt+G",
    pin: "Ctrl+Alt+P",
    capture: "Ctrl+Alt+Space",
    palette: "Ctrl+Alt+K",
    note: "Ctrl+Alt+J",
    pasteNext: "Ctrl+Alt+Shift+V",
    switcher: "Ctrl+Alt+W",
    blur: "Ctrl+Alt+B",
    explain: "Ctrl+Alt+E",
    rewrite: "Ctrl+Alt+R",
  },
  vault: { requireHello: false, clearSecs: 0, neverClear: false },
  dev: { widgets: [], noServerAlerts: false, noCodingTracker: false, claudeDailyCapUsd: 0 },
  desk: { routines: [], noHabitRoutines: false, stickies: [] },
  breaks: {
    noEyes: false,
    noStretch: false,
    noWater: true,
    eyeMin: 20,
    stretchMin: 60,
    waterMin: 90,
    day: "",
    count: 0,
    streak: 0,
    countedDay: "",
    best: 0,
  },
  fun: { petWalk: false, rgb: false, rgbPort: 0 },
  wallpaper: { target: "", recent: [] },
  power: { sleepIfIdleMin: 15, days: [1, 2, 3, 4, 5, 6, 7] },
  away: { afterIdleMin: 0, allScreens: false },
  layout: { density: "comfortable", panels: {} },
  pet: {
    name: "",
    color: "",
    tone: "cheerful",
    talk: true,
    chattiness: "normal",
    chatMinutes: 5,
    sayMeetings: true,
    sayBreaks: true,
    sayFocus: true,
    sayAgents: true,
    sayBattery: true,
    sayWeather: true,
    sayLate: true,
    sayGreeting: true,
    reactMusic: true,
    reactVideo: true,
    reactCoding: true,
    reactWeather: true,
    reactBattery: true,
    reactDay: true,
    petting: true,
    leave: "click",
    born: "",
    pats: 0,
    xp: 0,
    treats: 0,
    buddyOut: false,
    play: true,
    hunger: true,
    fullness: 80,
    friends: true,
    hat: "",
    streakDays: 0,
    lastPlayDay: "",
    fetchesToday: 0,
    fetchDay: "",
  },
  ai: { provider: "claude", models: {}, baseUrl: "" },
  day: {
    startMin: 540,
    endMin: 1080,
    workDays: [1, 2, 3, 4, 5],
    noAutoPlan: false,
    noWorkdayBar: false,
    shareGoogle: false,
    reviewAt: 1260,
    noReview: false,
    noBriefing: false,
  },
  gestures: { disabled: false, sensitivity: "normal", noTrackSwipe: false },
  dock: { order: [], hidden: [], profiles: [], active: "", auto: false },
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
