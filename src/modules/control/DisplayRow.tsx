// Control → display switches: night light, HDR (when a screen supports it), the refresh rate
// (click to step through what the screen offers) and the touchpad's settings.
import { Monitor, Moon, MousePointer2, Sun } from "lucide-react";
import { Fragment, useEffect, useState } from "react";

import type { DisplayToggles } from "../../core/generated/DisplayToggles";
import { ipc, isIpcError } from "../../core/ipc";
import { useArranged } from "../../shell/layout";
import { DISPLAY } from "./layout";

export function DisplayRow({ onError }: { onError: (text: string | null) => void }) {
  const [t, setT] = useState<DisplayToggles | null>(null);
  const shown = useArranged("control.display", DISPLAY);
  const read = () => ipc.displayToggles().then(setT, () => {});
  useEffect(() => {
    void read();
  }, []);
  const run = (p: Promise<unknown>) =>
    void p.then(
      () => {
        onError(null);
        window.setTimeout(() => void read(), 600);
      },
      (e) => onError(isIpcError(e) ? e.message.replace(/^invalid input: /, "") : "That didn't work."),
    );
  if (!t || shown.length === 0) return null;
  const next = t.rates.find((r) => r > t.hz) ?? t.rates[0];
  return (
    <div className="control-display" role="group" aria-label="Display">
      {shown.map(({ id }) => (
        <Fragment key={id}>{chip(id)}</Fragment>
      ))}
    </div>
  );

  function chip(id: string) {
    if (!t) return null;
    if (id === "night-light")
      return (
        <button
          type="button"
          className={`nd-chip${t.nightLight ? " nd-chip--on" : ""}`}
          aria-pressed={t.nightLight ?? undefined}
          title={
            t.nightLight === undefined
              ? "Opens Windows' Night light settings"
              : "Warmer colours for the evening"
          }
          onClick={() => run(ipc.setNightLight(!t.nightLight))}
        >
          <Moon size={13} aria-hidden /> Night light
        </button>
      );
    if (id === "hdr")
      return (
        t.hdr !== undefined && (
          <button
            type="button"
            className={`nd-chip${t.hdr ? " nd-chip--on" : ""}`}
            aria-pressed={t.hdr}
            onClick={() => run(ipc.setHdr(!t.hdr))}
          >
            <Sun size={13} aria-hidden /> HDR
          </button>
        )
      );
    if (id === "refresh")
      return (
        t.rates.length > 1 &&
        next !== undefined && (
          <button
            type="button"
            className="nd-chip"
            title={`Offered: ${t.rates.join(", ")} Hz — click for ${next} Hz`}
            onClick={() => run(ipc.setRefreshRate(next))}
          >
            <Monitor size={13} aria-hidden /> {t.hz} Hz
          </button>
        )
      );
    return (
      <button
        type="button"
        className="nd-chip"
        title="Turn the touchpad on or off in Windows Settings"
        onClick={() => void ipc.openTouchpadSettings().catch(() => {})}
      >
        <MousePointer2 size={13} aria-hidden /> Touchpad
      </button>
    );
  }
}
