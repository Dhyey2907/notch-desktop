// Plays the ambient mix in the notch whose mixer was used last (that click lets WebView2 play
// audio). Sounds don't resume by themselves when Notch starts.
import { useEffect, useRef } from "react";

import { windowLabel } from "../core/window";
import { useSettings } from "../settings";
import { AmbientEngine, SOUNDS, type Levels } from "./ambient";

const DEFAULT_MASTER = 0.6;

export function useAmbient() {
  const focus = useSettings((s) => s.settings?.focus);
  const engine = useRef<AmbientEngine | null>(null);
  const started = useRef(false);

  // Don't start playing on launch just because sounds were on when Notch last closed.
  useEffect(() => {
    if (started.current || !focus) return;
    started.current = true;
    if (focus.soundsOn && focus.soundOwner === windowLabel()) {
      void useSettings.getState().update((s) => ({ ...s, focus: { ...s.focus, soundsOn: false } }));
    }
  }, [focus]);

  useEffect(() => {
    if (!focus) return;
    engine.current ??= new AmbientEngine();
    const mine = focus.soundOwner === windowLabel();
    const levels = Object.fromEntries(SOUNDS.map((s) => [s, focus[s]])) as Levels;
    engine.current.apply(levels, focus.master || DEFAULT_MASTER, mine && focus.soundsOn);

    // Sleep timer.
    if (!mine || !focus.soundsOn || !focus.sleepUntilMs) return;
    const wait = focus.sleepUntilMs - Date.now();
    const stop = () =>
      void useSettings
        .getState()
        .update((s) => ({ ...s, focus: { ...s.focus, soundsOn: false, sleepUntilMs: undefined } }));
    if (wait <= 0) {
      stop();
      return;
    }
    const id = window.setTimeout(stop, wait);
    return () => window.clearTimeout(id);
  }, [focus]);

  useEffect(() => () => engine.current?.close(), []);
}
