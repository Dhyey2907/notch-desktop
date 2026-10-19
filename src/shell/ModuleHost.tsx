import { AnimatePresence, motion } from "motion/react";
import { Component, Suspense, type ErrorInfo, type ReactNode } from "react";

import { Button, ErrorState } from "../design/components";
import { CONTENT_SWAP, INSTANT } from "../design/motion";
import { useReduceMotion } from "../design/useReduceMotion";
import { findModule, MODULES } from "../modules/registry";
import { useLayoutEdit } from "./layout";
import { LayoutEditor } from "./LayoutEditor";
import { useEnabledModules, useModuleNav } from "./useModules";

/** Isolates a module's render errors so the shell and other modules keep working. */
export class ModuleErrorBoundary extends Component<
  { name: string; children: ReactNode },
  { failed: boolean }
> {
  state = { failed: false };

  static getDerivedStateFromError() {
    return { failed: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error(`module ${this.props.name} crashed`, error, info.componentStack);
  }

  render() {
    if (this.state.failed) {
      return (
        <ErrorState
          title={`${this.props.name} hit a problem`}
          body="Other modules keep working. Try reloading this one."
          action={<Button onClick={() => this.setState({ failed: false })}>Reload</Button>}
        />
      );
    }
    return this.props.children;
  }
}

/** Expanded view of the active module, with a direction-aware slide between modules. */
export function ModuleHost() {
  const activeId = useModuleNav((s) => s.activeId);
  const reduceMotion = useReduceMotion();
  const editing = useLayoutEdit((s) => s.editing);
  const module = findModule(activeId);
  if (!module) return null;
  const { Expanded } = module;

  return (
    <AnimatePresence mode="popLayout" initial={false}>
      <motion.div
        key={editing ? `${module.id}-layout` : module.id}
        className="module-host"
        initial={{ opacity: 0, x: 12 }}
        animate={{ opacity: 1, x: 0 }}
        exit={{ opacity: 0, x: -12 }}
        transition={reduceMotion ? INSTANT : CONTENT_SWAP}
      >
        <ModuleErrorBoundary name={module.name}>
          {editing ? (
            <LayoutEditor module={module} />
          ) : (
            <Suspense fallback={null}>
              <Expanded />
            </Suspense>
          )}
        </ModuleErrorBoundary>
      </motion.div>
    </AnimatePresence>
  );
}

/** Peek content: a module with something live (music, a running timer), else Home's greeting. */
// Every registered module's claim hook, in a fixed order that never changes at runtime.
const CLAIM_HOOKS = MODULES.map((m) => m.usePeekClaim ?? (() => false));

/** Which registered modules currently claim the peek (same index order as `MODULES`). */
function usePeekClaims(): boolean[] {
  // CLAIM_HOOKS is a module-level constant, so these hooks run in the same order on every
  // render — the invariant the rules of hooks protect.
  // eslint-disable-next-line react-hooks/rules-of-hooks
  return CLAIM_HOOKS.map((useClaim) => useClaim());
}

export function PeekHost() {
  const enabled = useEnabledModules();
  const claims = usePeekClaims();
  const claimant = MODULES.find((m, i) => claims[i] && enabled.includes(m));
  const Peek = claimant?.Peek ?? findModule("home")?.Peek;
  return Peek ? <Peek /> : null;
}

/** Collapsed indicators contributed by modules; each renders null when idle. */
export function CompactHost() {
  const withCompact = useEnabledModules().filter((m) => m.Compact);
  return (
    <div className="compact-host">
      {withCompact.map(({ id, Compact }) => Compact && <Compact key={id} />)}
    </div>
  );
}
