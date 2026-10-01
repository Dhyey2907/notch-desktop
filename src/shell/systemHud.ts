// Volume and brightness changes show a short HUD in the notch (also over fullscreen apps);
// Bluetooth devices connecting or disconnecting show a small card with their battery.
import { useEffect } from "react";

import { useActivities } from "../activity";
import type { ActivityInput, Hud } from "../activity/activityStore";
import { eventBus } from "../core/eventBus";
import { ipc } from "../core/ipc";
import { useSettings } from "../settings";

/** How long the HUD stays after the last change. */
const HUD_MS = 1600;
/** Bluetooth connect / disconnect cards. */
const BLUETOOTH_MS = 4000;
/** Dragging the bar produces many levels; send at most one this often. */
const SET_INTERVAL_MS = 30;

let lastSent = 0;
let queued: number | null = null;

function setVolume(level: number) {
  const now = performance.now();
  if (now - lastSent >= SET_INTERVAL_MS) {
    lastSent = now;
    void ipc.soundSetVolume(level).catch(() => {});
    return;
  }
  // Too soon: send the latest level once the interval has passed.
  if (queued === null) {
    window.setTimeout(() => {
      const next = queued;
      queued = null;
      if (next !== null) setVolume(next);
    }, SET_INTERVAL_MS);
  }
  queued = level;
}

function hudActivity(title: string, hud: Hud): ActivityInput {
  return {
    moduleId: "system",
    priority: "high",
    // One HUD at a time: a new change replaces it in place and restarts its timer.
    dedupKey: "system-hud",
    countRepeats: false,
    timeoutMs: HUD_MS,
    title,
    hud,
  };
}

export function useSystemHud() {
  useEffect(() => {
    const offSound = eventBus.on("SoundChanged", ({ event }) => {
      if (useSettings.getState().settings?.hud.hideVolume) return;
      const { state, switched } = event.payload;
      useActivities.getState().push(
        hudActivity(state.muted ? "Volume muted" : `Volume ${Math.round(state.level * 100)}%`, {
          kind: "volume",
          level: state.level,
          muted: state.muted,
          // Name the device when sound moves to it, or whenever it isn't the speakers.
          label: switched || state.device !== "Speakers" ? state.device : undefined,
          onSet: setVolume,
          onToggleMute: () => void ipc.soundToggleMute().catch(() => {}),
        }),
      );
    });
    const offBrightness = eventBus.on("BrightnessChanged", ({ event }) => {
      if (useSettings.getState().settings?.hud.hideBrightness) return;
      const { level } = event.payload;
      useActivities
        .getState()
        .push(hudActivity(`Brightness ${level}%`, { kind: "brightness", level: level / 100 }));
    });
    const offBluetooth = eventBus.on("BluetoothChanged", ({ event }) => {
      if (useSettings.getState().settings?.hud.hideBluetooth) return;
      const device = event.payload;
      useActivities.getState().push({
        moduleId: "system",
        priority: "normal",
        tone: device.connected ? "info" : "neutral",
        dedupKey: `bluetooth-${device.name}`,
        countRepeats: false,
        timeoutMs: BLUETOOTH_MS,
        avatar: device.name.trim().charAt(0).toUpperCase() || undefined,
        title: device.name,
        body: device.connected
          ? device.battery != null
            ? `Connected · ${device.battery}% battery`
            : "Connected"
          : "Disconnected",
      });
    });
    return () => {
      offSound();
      offBrightness();
      offBluetooth();
    };
  }, []);
}
