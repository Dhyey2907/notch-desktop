import { Settings } from "lucide-react";
import { motion } from "motion/react";
import type { KeyboardEvent } from "react";

import { ipc } from "../core/ipc";
import { IconButton } from "../design/components";
import { contentEnter } from "../design/motion";
import { useReduceMotion } from "../design/useReduceMotion";
import { useDockModules, useModuleNav } from "./useModules";

/**
 * Compact module switcher shown beneath the expanded island (design.md §17).
 * Arrow keys move between modules; the active one gets a raised treatment.
 */
export function Dock() {
  const modules = useDockModules();
  const activeId = useModuleNav((s) => s.activeId);
  const setActive = useModuleNav((s) => s.setActive);
  const step = useModuleNav((s) => s.step);
  const reduceMotion = useReduceMotion();

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
    e.preventDefault();
    step(e.key === "ArrowRight" ? 1 : -1);
    const id = useModuleNav.getState().activeId;
    e.currentTarget.querySelector<HTMLElement>(`[data-module="${id}"] button`)?.focus();
  };

  return (
    <motion.div
      className="dock"
      role="toolbar"
      aria-label="Modules"
      onKeyDown={onKeyDown}
      initial={{ opacity: 0, y: -6 }}
      animate={{ opacity: 1, y: 0, transition: contentEnter("expanded", reduceMotion) }}
      exit={{ opacity: 0, transition: { duration: 0.08 } }}
    >
      {modules.map((module) => (
        <span key={module.id} data-module={module.id}>
          <IconButton
            icon={module.icon}
            label={module.name}
            active={module.id === activeId}
            onClick={() => setActive(module.id)}
          />
        </span>
      ))}
      <span className="dock__divider" aria-hidden />
      <IconButton icon={Settings} label="Settings" onClick={() => void ipc.openSettingsWindow()} />
    </motion.div>
  );
}
