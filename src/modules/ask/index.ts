// Ask: quick questions answered by the user's own Claude Code (no API key), optionally about
// the window they were looking at. Ctrl+Alt+A opens it from anywhere. The conversation
// lives in memory only.
import { MessageCircleQuestion } from "lucide-react";
import { lazy } from "react";

import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { AskPeek } from "./AskPeek";
import { useAsk } from "./askStore";

function init({ events }: ModuleContext) {
  const offChanged = events.on("AskChanged", ({ event }) => useAsk.getState().set(event.payload));
  const offDelta = events.on("AskDelta", ({ event }) =>
    useAsk.getState().append(event.payload.id, event.payload.text),
  );
  ipc
    .askSnapshot()
    .then((snapshot) => {
      if (!useAsk.getState().snapshot) useAsk.getState().set(snapshot);
    })
    .catch(() => {});
  return () => {
    offChanged();
    offDelta();
  };
}

export const askModule: NotchModule = {
  id: "ask",
  name: "Ask",
  icon: MessageCircleQuestion,
  order: 5,
  init,
  Peek: AskPeek,
  Expanded: lazy(() => import("./AskExpanded")),
};
