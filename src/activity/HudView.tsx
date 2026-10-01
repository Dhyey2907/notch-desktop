// Volume / brightness readout: icon, level bar, percent. When the level can be changed from
// the notch, dragging or scrolling on the bar sets it and the icon toggles mute.
import { ArrowBigUp, Hash, Languages, Sun, SunDim, Volume1, Volume2, VolumeX } from "lucide-react";
import { useRef, type PointerEvent, type WheelEvent } from "react";

import type { Hud } from "./activityStore";

/** Scroll step, as a fraction of the full range. */
const WHEEL_STEP = 0.02;

function HudIcon({ hud }: { hud: Hud }) {
  if (hud.kind === "brightness") return hud.level < 0.4 ? <SunDim size={16} /> : <Sun size={16} />;
  if (hud.muted || hud.level === 0) return <VolumeX size={16} />;
  return hud.level < 0.5 ? <Volume1 size={16} /> : <Volume2 size={16} />;
}

const clamp = (level: number) => Math.min(1, Math.max(0, level));

/** A one-line state: "Caps Lock on", "English (United States)". */
function StateHud({ hud }: { hud: Hud }) {
  const Icon = hud.kind === "caps" ? ArrowBigUp : hud.kind === "numlock" ? Hash : Languages;
  return (
    <div className="hud hud--state" role="status">
      <span className={`hud__icon${hud.level > 0 ? " hud__icon--on" : ""}`} aria-hidden>
        <Icon size={16} />
      </span>
      <span className="hud__state">{hud.label}</span>
    </div>
  );
}

export function HudView({ hud }: { hud: Hud }) {
  if (hud.kind === "caps" || hud.kind === "numlock" || hud.kind === "language") return <StateHud hud={hud} />;
  return <LevelHud hud={hud} />;
}

function LevelHud({ hud }: { hud: Hud }) {
  const bar = useRef<HTMLDivElement>(null);
  const percent = Math.round(hud.level * 100);
  const name = hud.kind === "volume" ? "Volume" : "Brightness";
  const shown = hud.muted ? 0 : hud.level;

  const setFrom = (e: PointerEvent) => {
    const rect = bar.current?.getBoundingClientRect();
    if (!rect || !hud.onSet) return;
    hud.onSet(clamp((e.clientX - rect.left) / rect.width));
  };

  return (
    <div
      className="hud"
      role="group"
      aria-label={`${name} ${hud.muted ? "muted" : `${percent}%`}${hud.label ? `, ${hud.label}` : ""}`}
      onWheel={(e: WheelEvent) => {
        if (!hud.onSet) return;
        hud.onSet(clamp(hud.level + (e.deltaY < 0 ? WHEEL_STEP : -WHEEL_STEP)));
      }}
    >
      {hud.onToggleMute ? (
        <button
          type="button"
          className="hud__icon hud__icon--button"
          aria-label={hud.muted ? "Unmute" : "Mute"}
          title={hud.muted ? "Unmute" : "Mute"}
          onClick={hud.onToggleMute}
        >
          <HudIcon hud={hud} />
        </button>
      ) : (
        <span className="hud__icon" aria-hidden>
          <HudIcon hud={hud} />
        </span>
      )}
      <div
        ref={bar}
        className={`hud__bar${hud.onSet ? " hud__bar--interactive" : ""}`}
        role={hud.onSet ? "slider" : "meter"}
        aria-label={name}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percent}
        tabIndex={hud.onSet ? 0 : undefined}
        onKeyDown={(e) => {
          if (!hud.onSet) return;
          if (e.key === "ArrowRight" || e.key === "ArrowUp") hud.onSet(clamp(hud.level + WHEEL_STEP * 2));
          if (e.key === "ArrowLeft" || e.key === "ArrowDown") hud.onSet(clamp(hud.level - WHEEL_STEP * 2));
        }}
        onPointerDown={(e) => {
          if (!hud.onSet) return;
          e.currentTarget.setPointerCapture(e.pointerId);
          setFrom(e);
        }}
        onPointerMove={(e) => {
          if (e.buttons & 1) setFrom(e);
        }}
      >
        <span className="hud__fill" style={{ width: `${shown * 100}%` }} />
      </div>
      <span className="hud__value">{hud.muted ? "Muted" : percent}</span>
      {hud.label && <span className="hud__label">{hud.label}</span>}
    </div>
  );
}
