// Module contract (architecture.md §5). Modules own their UI and state and talk to the rest
// of the app only through this context and the event bus — never to each other.
import type { LucideIcon } from "lucide-react";
import type { ComponentType } from "react";

import type { ActivityState } from "../activity/activityStore";
import type { EventBus } from "../core/eventBus";

export interface ModuleContext {
  events: Pick<EventBus, "on" | "onAny">;
  /** Ask the shell to show something; the activity engine decides when and how. */
  activities: Pick<ActivityState, "push" | "dismiss" | "update">;
}

export interface NotchModule {
  /** Stable identifier, used for settings and navigation. */
  id: string;
  name: string;
  icon: LucideIcon;
  /** Dock position; lower comes first. */
  order: number;
  /** Tiny indicator for the collapsed island. Must render `null` when there is nothing to show. */
  Compact?: ComponentType;
  /** Short preview for the peek state. */
  Peek?: ComponentType;
  /**
   * Hook: true while the module has something live (e.g. music playing). Hovering the notch
   * then previews this module instead of the last one opened.
   */
  usePeekClaim?: () => boolean;
  /** Main view. Prefer `React.lazy` so module code loads on first open. */
  Expanded: ComponentType;
  /** Start background subscriptions once the module is enabled; return a disposer. */
  init?: (ctx: ModuleContext) => void | (() => void);
  /** Development-only modules are excluded from release builds. */
  devOnly?: boolean;
  /** Cannot be disabled in Settings (e.g. Home, which hosts first-run and guidance). */
  alwaysOn?: boolean;
  /** Never in the dock or the dock settings; opened another way (the command palette). */
  dockless?: boolean;
}
