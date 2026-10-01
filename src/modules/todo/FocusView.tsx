// Focus tab: start a session (cards wait until it ends) and mix ambient sounds.
import { CloudRain, Flame, Pause, Play, Square, Waves, Wind, type LucideIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import type { FocusSettings } from "../../core/generated/FocusSettings";
import { ipc, isIpcError } from "../../core/ipc";
import { windowLabel } from "../../core/window";
import { Button, SegmentedControl, Switch } from "../../design/components";
import { SOUNDS, type Sound } from "../../focus/ambient";
import { focusActive, focusProgress, useFocus } from "../../focus/focusStore";
import { useSettings } from "../../settings";
import { formatCountdown } from "./parse";
import { useSecondClock } from "./useSecondClock";

const LENGTHS = [25, 50, 90];
const SAVE_DELAY_MS = 150;

const SOUND_INFO: Record<Sound, { label: string; icon?: LucideIcon }> = {
  rain: { label: "Rain", icon: CloudRain },
  wind: { label: "Wind", icon: Wind },
  ocean: { label: "Ocean", icon: Waves },
  fire: { label: "Fire", icon: Flame },
  brown: { label: "Brown noise" },
  pink: { label: "Pink noise" },
};

type Sleep = "off" | "15" | "30" | "60";

function message(error: unknown) {
  return isIpcError(error) ? error.message.replace(/^invalid input: /, "") : "Something went wrong.";
}

export function FocusView() {
  return (
    <div className="focus">
      <Session />
      <Sounds />
    </div>
  );
}

function Session() {
  const state = useFocus((s) => s.state);
  const now = useSecondClock(Boolean(state?.endsAt));
  const active = focusActive(state, now);
  const [error, setError] = useState<string | null>(null);
  const run = (p: Promise<void>) =>
    void p.then(
      () => setError(null),
      (e) => setError(message(e)),
    );
  const progress = focusProgress(state, now);
  const r = 34;
  const c = 2 * Math.PI * r;

  return (
    <section className="focus__session" aria-label="Focus session">
      <div className="focus__ring">
        <svg viewBox="0 0 80 80" aria-hidden>
          <circle cx="40" cy="40" r={r} className="focus__track" />
          <circle
            cx="40"
            cy="40"
            r={r}
            className="focus__value"
            strokeDasharray={c}
            strokeDashoffset={c * (1 - (active ? progress : 0))}
          />
        </svg>
        <span className="focus__time">
          {active ? formatCountdown(Date.parse(state!.endsAt!) - now) : "Focus"}
        </span>
      </div>
      {active ? (
        <Button onClick={() => run(ipc.focusStop())}>
          <Square size={12} aria-hidden /> Stop
        </Button>
      ) : (
        <div className="focus__lengths" role="group" aria-label="Start focusing for">
          {LENGTHS.map((m) => (
            <button key={m} type="button" className="todo-chip" onClick={() => run(ipc.focusStart(m))}>
              {m}m
            </button>
          ))}
        </div>
      )}
      <p className="focus__note">
        {error ??
          (active
            ? "Notifications wait until you're done."
            : state?.doneToday
              ? `${state.doneToday} ${state.doneToday === 1 ? "session" : "sessions"} today`
              : "Cards wait while you focus.")}
      </p>
    </section>
  );
}

/** Settings writes are batched while a slider is dragged. */
function useFocusSettings() {
  const saved = useSettings((s) => s.settings?.focus);
  const [draft, setDraft] = useState<FocusSettings | null>(null);
  const timer = useRef<number | undefined>(undefined);
  const value = draft ?? saved;

  useEffect(() => () => window.clearTimeout(timer.current), []);

  const change = (patch: Partial<FocusSettings>) => {
    if (!value) return;
    // Using the mixer makes this screen's notch the one that plays.
    const next = { ...value, ...patch, soundOwner: windowLabel() };
    setDraft(next);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      void useSettings
        .getState()
        .update((s) => ({ ...s, focus: next }))
        .finally(() => setDraft(null));
    }, SAVE_DELAY_MS);
  };
  return { value, change };
}

function Sounds() {
  const { value, change } = useFocusSettings();
  const now = useSecondClock(Boolean(value?.sleepUntilMs));
  if (!value) return null;
  const anyUp = SOUNDS.some((s) => value[s] > 0);
  const playing = value.soundsOn && anyUp && value.soundOwner === windowLabel();
  const sleep: Sleep = value.sleepUntilMs
    ? (String(Math.max(15, Math.round((value.sleepUntilMs - now) / 60_000 / 15) * 15)) as Sleep)
    : "off";

  return (
    <section className="focus__sounds" aria-label="Ambient sounds">
      <div className="focus__sounds-head">
        <button
          type="button"
          className="focus__play"
          aria-label={playing ? "Pause sounds" : "Play sounds"}
          disabled={!anyUp}
          onClick={() => change({ soundsOn: !playing })}
        >
          {playing ? <Pause size={13} aria-hidden /> : <Play size={13} aria-hidden />}
        </button>
        <span className="focus__sounds-title">Sounds</span>
        <input
          className="focus__slider focus__slider--master"
          type="range"
          min={0}
          max={1}
          step={0.01}
          aria-label="Sounds volume"
          value={value.master || 0.6}
          onChange={(e) => change({ master: Number(e.target.value) })}
        />
      </div>
      <div className="focus__mixer">
        {SOUNDS.map((sound) => {
          const Icon = SOUND_INFO[sound].icon;
          return (
            <label key={sound} className="focus__channel">
              <span className="focus__channel-name">
                {Icon && <Icon size={12} aria-hidden />} {SOUND_INFO[sound].label}
              </span>
              <input
                className="focus__slider"
                type="range"
                min={0}
                max={1}
                step={0.01}
                value={value[sound]}
                onChange={(e) => {
                  const level = Number(e.target.value);
                  // Turning a sound up starts playback.
                  change({ [sound]: level, soundsOn: level > 0 ? true : value.soundsOn });
                }}
              />
            </label>
          );
        })}
      </div>
      <div className="focus__foot">
        <span className="focus__sleep-label">Sleep</span>
        <SegmentedControl<Sleep>
          label="Stop sounds after"
          value={sleep}
          options={[
            { value: "off", label: "Off" },
            { value: "15", label: "15m" },
            { value: "30", label: "30m" },
            { value: "60", label: "60m" },
          ]}
          onChange={(v) =>
            change({ sleepUntilMs: v === "off" ? undefined : Date.now() + Number(v) * 60_000 })
          }
        />
        <Switch label="Pet" checked={!value.hidePet} onChange={(on) => change({ hidePet: !on })} />
      </div>
    </section>
  );
}
