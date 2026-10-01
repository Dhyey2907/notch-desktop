// Settings building blocks beyond the design system: a labelled row with an anchor (for
// search), a slider, colour swatches, a shortcut recorder, a time field and a dropdown.
import { Check, RotateCcw } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";

import { settingAnchor } from "./searchIndex";
import { prettyShortcut, shortcutFrom } from "./shortcutText";

/** One setting: plain-language label and description on the left, its control on the right. */
export function Row({
  label,
  description,
  children,
  stacked,
}: {
  label: string;
  description?: ReactNode;
  children: ReactNode;
  /** Put the control under the text (for wide controls). */
  stacked?: boolean;
}) {
  return (
    <div id={settingAnchor(label)} className={`settings__row${stacked ? " settings__row--stacked" : ""}`}>
      <span className="settings__row-text">
        <span className="settings__row-label">{label}</span>
        {description && <span className="settings__row-description">{description}</span>}
      </span>
      {children}
    </div>
  );
}

export function Slider({
  label,
  value,
  min,
  max,
  step = 1,
  onChange,
  format = (v) => String(v),
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (value: number) => void;
  format?: (value: number) => string;
}) {
  // Follow the thumb while dragging; save when it's let go.
  const [draft, setDraft] = useState<number | null>(null);
  const shown = draft ?? value;
  const commit = () => {
    if (draft !== null && draft !== value) onChange(draft);
    setDraft(null);
  };
  return (
    <span className="settings-slider">
      <input
        type="range"
        aria-label={label}
        min={min}
        max={max}
        step={step}
        value={shown}
        style={{ "--fill": `${((shown - min) / (max - min)) * 100}%` } as React.CSSProperties}
        onChange={(e) => setDraft(Number(e.currentTarget.value))}
        onPointerUp={commit}
        onKeyUp={commit}
        onBlur={commit}
      />
      <span className="settings-slider__value">{format(shown)}</span>
    </span>
  );
}

const SWATCHES = ["", "#60a5fa", "#a78bfa", "#f472b6", "#f87171", "#fb923c", "#facc15", "#4ade80", "#2dd4bf"];

export function ColorChoice({ value, onChange }: { value: string; onChange: (hex: string) => void }) {
  return (
    <span className="settings-colors" role="radiogroup" aria-label="Accent color">
      {SWATCHES.map((hex) => (
        <button
          key={hex || "default"}
          type="button"
          role="radio"
          aria-checked={value === hex}
          aria-label={hex ? hex : "White (default)"}
          title={hex ? hex : "White (default)"}
          className="settings-colors__swatch"
          style={{ background: hex || "#ffffff" }}
          onClick={() => onChange(hex)}
        >
          {value === hex && <Check size={12} aria-hidden />}
        </button>
      ))}
      <label className="settings-colors__custom" title="Pick any color">
        <input
          type="color"
          aria-label="Custom accent color"
          value={value || "#ffffff"}
          onChange={(e) => onChange(e.currentTarget.value)}
        />
      </label>
    </span>
  );
}

export function ShortcutRecorder({
  label,
  value,
  fallback,
  onChange,
}: {
  label: string;
  value: string;
  fallback: string;
  onChange: (value: string) => void;
}) {
  const [recording, setRecording] = useState(false);

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") return setRecording(false);
      if (e.key === "Backspace" || e.key === "Delete") {
        onChange("");
        return setRecording(false);
      }
      const shortcut = shortcutFrom(e);
      if (!shortcut) return;
      onChange(shortcut);
      setRecording(false);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, onChange]);

  return (
    <span className="settings-shortcut">
      <button
        type="button"
        className={`settings__kbd settings-shortcut__keys${recording ? " settings-shortcut__keys--recording" : ""}`}
        aria-label={`${label}: ${value ? prettyShortcut(value) : "off"}. Click to change.`}
        onClick={() => setRecording((r) => !r)}
        onBlur={() => setRecording(false)}
      >
        {recording ? "Press keys…" : value ? prettyShortcut(value) : "Off"}
      </button>
      {value !== fallback && !recording && (
        <button
          type="button"
          className="settings-shortcut__reset"
          aria-label={`Reset ${label} to ${prettyShortcut(fallback)}`}
          title={`Back to ${prettyShortcut(fallback)}`}
          onClick={() => onChange(fallback)}
        >
          <RotateCcw size={13} aria-hidden />
        </button>
      )}
    </span>
  );
}

export function TimeField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <input
      type="time"
      className="settings-time"
      aria-label={label}
      value={value}
      onChange={(e) => e.currentTarget.value && onChange(e.currentTarget.value)}
    />
  );
}

export function Select<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
}) {
  return (
    <select
      className="settings-select"
      aria-label={label}
      value={value}
      onChange={(e) => onChange(e.currentTarget.value as T)}
    >
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}
