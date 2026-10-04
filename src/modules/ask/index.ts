// Ask: quick questions answered by the AI chosen in Settings → AI (Claude Code by default),
// optionally about the window they were looking at. Ctrl+Alt+A opens it from anywhere. The conversation
// lives in memory only.
import { MessageCircleQuestion } from "lucide-react";
import { lazy } from "react";

import { ipc } from "../../core/ipc";
import { useSettings } from "../../settings";
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
  // A different AI was picked in Settings → AI: show its name and whether it's ready.
  const current = useSettings.getState().settings?.ai;
  let provider: string | null = current ? JSON.stringify(current) : null;
  const offSettings = events.on("SettingsChanged", ({ event }) => {
    const ai = JSON.stringify(event.payload.ai);
    if (provider !== ai)
      ipc
        .askSnapshot()
        .then((s) => useAsk.getState().set(s))
        .catch(() => {});
    provider = ai;
  });
  return () => {
    offChanged();
    offDelta();
    offSettings();
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
