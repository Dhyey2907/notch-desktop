import { holdDuringFocus } from "../focus/focusStore";
import { createActivityStore } from "./activityStore";

/** The app-wide activity queue. Modules reach it through `ModuleContext.activities`.
 * During a focus session, non-urgent activities are held back (see `holdDuringFocus`). */
export const useActivities = createActivityStore({ hold: holdDuringFocus });
