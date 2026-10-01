// Mail: a glance at your Gmail inbox — unread count, the latest unread messages, Archive and
// Mark read. Uses the Google sign-in from Calendar with the Gmail permission turned on.
import { Mail } from "lucide-react";
import { lazy } from "react";

import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { MailPeek } from "./MailPeek";
import { threadUrl, useMail } from "./mailStore";

function init({ events, activities }: ModuleContext) {
  const offChanged = events.on("MailChanged", ({ event }) => useMail.setState({ state: event.payload }));
  const offNew = events.on("NewMail", ({ event }) => {
    const m = event.payload;
    activities.push({
      moduleId: "mail",
      priority: "normal",
      tone: "info",
      dedupKey: `mail-${m.id}`,
      countRepeats: false,
      avatar: m.from.charAt(0).toUpperCase() || undefined,
      title: m.from,
      body: m.subject,
      actions: [
        { id: "archive", label: "Archive", variant: "secondary", run: () => ipc.mailArchive(m.id) },
        { id: "open", label: "Open", variant: "primary", run: () => ipc.openExternal(threadUrl(m.threadId)) },
      ],
    });
  });
  ipc.mailState().then(
    (state) => useMail.setState({ state }),
    () => {},
  );
  return () => {
    offChanged();
    offNew();
  };
}

export const mailModule: NotchModule = {
  id: "mail",
  name: "Mail",
  icon: Mail,
  order: 22,
  init,
  Peek: MailPeek,
  Expanded: lazy(() => import("./MailExpanded")),
};
