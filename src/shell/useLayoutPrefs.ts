// Applies Edit layout's choices: the open panel's size and the spacing for every panel. Ends
// layout editing when the notch closes.
import { useEffect } from "react";

import { useSettings } from "../settings";
import { PANEL_SIZES, useLayoutEdit } from "./layout";
import { useIsland } from "./useIsland";
import { useModuleNav } from "./useModules";

export function useLayoutPrefs() {
  const activeId = useModuleNav((s) => s.activeId);
  const size = useSettings((s) => s.settings?.layout?.panels[activeId ?? ""]?.size ?? "normal");
  const density = useSettings((s) => s.settings?.layout?.density ?? "comfortable");
  const mode = useIsland((s) => s.mode);

  useEffect(() => {
    void useIsland.getState().setExpandedSize(PANEL_SIZES[size]);
  }, [size]);

  useEffect(() => {
    document.documentElement.setAttribute("data-density", density);
  }, [density]);

  useEffect(() => {
    if (mode !== "expanded") useLayoutEdit.getState().set(false);
  }, [mode]);
}
