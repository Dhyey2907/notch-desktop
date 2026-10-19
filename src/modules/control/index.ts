// Control center: quick switches (Wi-Fi, Bluetooth, hotspot…), the network with live speed,
// the battery's health and charge time, and accessory batteries. Its alerts: internet down
// or back, "unplug at 80%", charging, a low accessory, Caps Lock and keyboard language.
import { SlidersHorizontal } from "lucide-react";
import { lazy } from "react";

import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { useControl } from "./controlStore";
import { alertCard } from "./format";
import { CONTROL_LAYOUT } from "./layout";

/** Lock-key and language readouts, like the volume HUD. */
const STATE_HUD_MS = 1300;
const CHARGING_GLOW_MS = 4000;
const DISK_FIRST_MS = 60_000;
const DISK_EVERY_MS = 24 * 60 * 60_000;

/** A soft green glow around the notch for a few seconds (island.css `[data-charging]`). */
function glowCharging() {
  const root = document.documentElement;
  root.setAttribute("data-charging", "");
  window.setTimeout(() => root.removeAttribute("data-charging"), CHARGING_GLOW_MS);
}

function init({ events, activities }: ModuleContext) {
  const offChanged = events.on("ControlChanged", ({ event }) => useControl.getState().set(event.payload));
  const offAlert = events.on("SystemAlert", ({ event }) => {
    const card = alertCard(event.payload);
    if (card.hud) {
      activities.push({
        moduleId: "control",
        priority: "high",
        dedupKey: "system-hud",
        countRepeats: false,
        timeoutMs: STATE_HUD_MS,
        title: card.title,
        hud: { kind: card.hud.kind, level: card.hud.on ? 1 : 0, label: card.title },
      });
      return;
    }
    const kind = event.payload.kind;
    // Charging: a soft green glow around the notch for a moment.
    if (kind === "plugged") glowCharging();
    activities.push({
      moduleId: "control",
      priority: kind === "batteryLow" ? "high" : "normal",
      tone: card.tone,
      // Down and back replace each other; other kinds stand alone.
      dedupKey: kind === "internetDown" || kind === "internetBack" ? "internet" : `control-${kind}`,
      countRepeats: false,
      title: card.title,
      body: card.body,
    });
  });
  // Drive health once a minute after start, then daily: one card per problem per day.
  const checkDisks = () =>
    ipc.diskHealth(true).then(
      (d) => {
        if (!d.problem) return;
        const today = new Date().toLocaleDateString("en-CA");
        const key = `overline.disk:${today}:${d.problem}`;
        try {
          if (localStorage.getItem(key)) return;
          localStorage.setItem(key, "1");
        } catch {
          // Storage off: warn anyway.
        }
        activities.push({
          moduleId: "control",
          priority: "normal",
          tone: "warning",
          dedupKey: "control-disk",
          countRepeats: false,
          title: "Check your drives",
          body: d.problem,
        });
      },
      () => {},
    );
  const firstDisk = window.setTimeout(() => void checkDisks(), DISK_FIRST_MS);
  const dailyDisk = window.setInterval(() => void checkDisks(), DISK_EVERY_MS);
  ipc.controlSnapshot().then(
    (snapshot) => useControl.getState().set(snapshot),
    () => {},
  );
  return () => {
    offChanged();
    offAlert();
    window.clearTimeout(firstDisk);
    window.clearInterval(dailyDisk);
  };
}

export const controlModule: NotchModule = {
  id: "control",
  name: "Control",
  icon: SlidersHorizontal,
  order: 2,
  init,
  Expanded: lazy(() => import("./ControlExpanded")),
  layout: CONTROL_LAYOUT,
};
