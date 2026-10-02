// Static module registry. Settings-driven enable/disable arrives in M4; until then every
// registered module is enabled.
import { askModule } from "./ask";
import { calendarModule } from "./calendar";
import { claudeModule } from "./claude";
import { clipboardModule } from "./clipboard";
import { controlModule } from "./control";
import { devEventsModule } from "./devEvents";
import { dayModule } from "./day";
import { deskModule } from "./desk";
import { devToolsModule } from "./devtools";
import { mailModule } from "./mail";
import { galleryModule } from "./gallery";
import { gitModule } from "./git";
import { homeModule } from "./home";
import { mediaModule } from "./media";
import { phoneModule } from "./phone";
import { todoModule } from "./todo";
import { vaultModule } from "./vault";
import type { NotchModule } from "./types";

export function sortModules(modules: NotchModule[], includeDev: boolean): NotchModule[] {
  return modules.filter((m) => includeDev || !m.devOnly).sort((a, b) => a.order - b.order);
}

// `import.meta.env.DEV` is a compile-time constant, so dev-only modules are tree-shaken
// out of release bundles entirely (not just hidden). Even in dev builds they stay out of
// the dock unless asked for with VITE_NOTCH_DEV_MODULES=1.
const devModules: NotchModule[] =
  import.meta.env.DEV && import.meta.env.VITE_NOTCH_DEV_MODULES === "1"
    ? [galleryModule, devEventsModule]
    : [];

export const MODULES: readonly NotchModule[] = sortModules(
  [
    homeModule,
    dayModule,
    mailModule,
    controlModule,
    askModule,
    mediaModule,
    calendarModule,
    gitModule,
    devToolsModule,
    deskModule,
    claudeModule,
    todoModule,
    clipboardModule,
    vaultModule,
    phoneModule,
    ...devModules,
  ],
  import.meta.env.DEV,
);

export function findModule(id: string): NotchModule | undefined {
  return MODULES.find((m) => m.id === id);
}
