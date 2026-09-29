import type { ActivityInput } from "../../activity/activityStore";
import type { ModuleContext } from "../types";

// The module's activity channel, set while the module is initialized so views can report
// operation results even after the island collapses.
let activities: ModuleContext["activities"] | null = null;

export function bindActivities(channel: ModuleContext["activities"] | null) {
  activities = channel;
}

export function notify(input: Omit<ActivityInput, "moduleId">) {
  activities?.push({ moduleId: "git", ...input });
}
