import { createActivityStore } from "./activityStore";

/** The app-wide activity queue. Modules reach it through `ModuleContext.activities`. */
export const useActivities = createActivityStore();
