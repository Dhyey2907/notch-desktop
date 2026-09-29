// Phone module: your phone's notifications (through Microsoft Phone Link) in the notch.
// Replying happens in Phone Link — Windows offers no way to reply through another app's
// notification — so "Reply" brings Phone Link up.
import { Smartphone } from "lucide-react";
import { lazy } from "react";

import type { PhoneNotification } from "../../core/generated/PhoneNotification";
import type { PhoneSnapshot } from "../../core/generated/PhoneSnapshot";
import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { PhonePeek } from "./PhoneIndicators";
import { usePhone } from "./phoneStore";

function init({ events, activities }: ModuleContext) {
  // Pop-up cards on this screen, by notification id.
  const shown = new Map<number, string>();

  const show = (n: PhoneNotification) => {
    const id = activities.push({
      moduleId: "phone",
      priority: "normal",
      tone: "info",
      dedupKey: `phone-${n.id}`,
      countRepeats: false,
      avatar: n.title.trim().charAt(0).toUpperCase() || undefined,
      title: n.title,
      body: n.body || "Phone Link",
      actions: [
        { id: "dismiss", label: "Dismiss", variant: "secondary", run: () => ipc.phoneDismiss(n.id) },
        { id: "reply", label: "Reply", variant: "primary", run: () => ipc.phoneOpen() },
      ],
    });
    shown.set(n.id, id);
  };

  const apply = (snapshot: PhoneSnapshot) => {
    usePhone.getState().set(snapshot);
    // Cleared on the phone or in Action Center: drop its card too.
    const live = new Set(snapshot.notifications.map((n) => n.id));
    for (const [notificationId, activityId] of shown) {
      if (!live.has(notificationId)) {
        activities.dismiss(activityId);
        shown.delete(notificationId);
      }
    }
  };

  const offChanged = events.on("PhoneChanged", ({ event }) => apply(event.payload));
  const offPosted = events.on("PhoneNotification", ({ event }) => show(event.payload));
  ipc
    .phoneSnapshot()
    .then((snapshot) => {
      if (!usePhone.getState().snapshot) apply(snapshot);
    })
    .catch((error) => console.error("failed to read phone notifications", error));
  return () => {
    offChanged();
    offPosted();
  };
}

export const phoneModule: NotchModule = {
  id: "phone",
  name: "Phone",
  icon: Smartphone,
  order: 60,
  init,
  Peek: PhonePeek,
  Expanded: lazy(() => import("./PhoneExpanded")),
};
