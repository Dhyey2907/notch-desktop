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
import { useSettings } from "../../settings";
import { caller, callKind } from "./calls";
import { findCode, masked } from "./codes";
import { usePhone } from "./phoneStore";

/** A copied sign-in code is cleared after this long. */
const CODE_CLEAR_SECS = 30;

function init({ events, activities }: ModuleContext) {
  // Pop-up cards on this screen, by notification id.
  const shown = new Map<number, string>();

  const show = (n: PhoneNotification) => {
    const call = callKind(n.title, n.body);
    if (call) {
      const who = caller(n.title, n.body);
      shown.set(
        n.id,
        activities.push({
          moduleId: "phone",
          priority: call === "incoming" ? "high" : "normal",
          tone: call === "incoming" ? "info" : "warning",
          dedupKey: `phone-call-${n.id}`,
          countRepeats: false,
          avatar: who.charAt(0).toUpperCase(),
          title: call === "incoming" ? `${who} is calling` : `Missed call · ${who}`,
          body:
            call === "incoming"
              ? "Answer on your phone or in Phone Link"
              : "Call back from your phone or Phone Link",
          timeoutMs: call === "incoming" ? 45_000 : undefined,
          actions: [
            { id: "dismiss", label: "Dismiss", variant: "secondary", run: () => ipc.phoneDismiss(n.id) },
            {
              id: "open",
              label: call === "incoming" ? "Open Phone Link" : "Call back",
              variant: "primary",
              run: () => ipc.phoneOpen(),
            },
          ],
        }),
      );
      return;
    }
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
  // A sign-in code: copy it (privately: not in history) and say so.
  const copyCode = (n: PhoneNotification) => {
    if (useSettings.getState().settings?.clipboard.noPhoneCodes) return;
    const code = findCode(n.title, n.body);
    if (!code) return;
    ipc.clipboardCopyPrivate(code, CODE_CLEAR_SECS).then(
      () =>
        activities.push({
          moduleId: "phone",
          priority: "high",
          tone: "success",
          dedupKey: "phone-code",
          countRepeats: false,
          private: true,
          title: `Code ${masked(code)} copied`,
          body: `From ${n.title || "your phone"} · paste it now; it clears in ${CODE_CLEAR_SECS}s`,
        }),
      () => {},
    );
  };
  const offPosted = events.on("PhoneNotification", ({ event }) => {
    show(event.payload);
    copyCode(event.payload);
  });
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
