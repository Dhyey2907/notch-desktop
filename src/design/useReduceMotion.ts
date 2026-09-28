import { useReducedMotion } from "motion/react";

import { useSettings } from "../settings";

/**
 * True when motion should be minimized: the Windows "animation effects" preference is off,
 * or the user chose "Reduced" in Settings → Appearance.
 */
export function useReduceMotion(): boolean {
  const system = useReducedMotion();
  const preference = useSettings((s) => s.settings?.animation);
  return preference === "reduced" || Boolean(system);
}
