// Control center: quick switches (Wi-Fi, Bluetooth, hotspot…), the network with live speed,
// the battery's health and charge time, and accessory batteries. Its alerts: internet down
// or back, "unplug at 80%", charging, a low accessory, Caps Lock and keyboard language.
import { SlidersHorizontal } from "lucide-react";
import { lazy } from "react";

import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { useControl } from "./controlStore";
import { alertCard } from "./format";

/** Lock-key and language readouts, like the volume HUD. */
const STATE_HUD_MS = 1300;

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
    activities.push({
      moduleId: "control",
      priority: "normal",
      tone: card.tone,
      // Down and back replace each other; other kinds stand alone.
      dedupKey: kind === "internetDown" || kind === "internetBack" ? "internet" : `control-${kind}`,
      countRepeats: false,
      title: card.title,
      body: card.body,
    });
  });
  ipc.controlSnapshot().then(
    (snapshot) => useControl.getState().set(snapshot),
    () => {},
  );
  return () => {
    offChanged();
    offAlert();
  };
}

export const controlModule: NotchModule = {
  id: "control",
  name: "Control",
  icon: SlidersHorizontal,
  order: 2,
  init,
  Expanded: lazy(() => import("./ControlExpanded")),
};
