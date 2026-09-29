// Static module registry. Settings-driven enable/disable arrives in M4; until then every
// registered module is enabled.
import { calendarModule } from "./calendar";
import { devEventsModule } from "./devEvents";
import { galleryModule } from "./gallery";
import { gitModule } from "./git";
import { homeModule } from "./home";
import { mediaModule } from "./media";
import type { NotchModule } from "./types";

export function sortModules(modules: NotchModule[], includeDev: boolean): NotchModule[] {
  return modules.filter((m) => includeDev || !m.devOnly).sort((a, b) => a.order - b.order);
}

// `import.meta.env.DEV` is a compile-time constant, so dev-only modules are tree-shaken
// out of release bundles entirely (not just hidden).
const devModules: NotchModule[] = import.meta.env.DEV ? [galleryModule, devEventsModule] : [];

export const MODULES: readonly NotchModule[] = sortModules(
  [homeModule, mediaModule, calendarModule, gitModule, ...devModules],
  import.meta.env.DEV,
);

export function findModule(id: string): NotchModule | undefined {
  return MODULES.find((m) => m.id === id);
}
