// Design-system primitives (plan.md Phase 3). Modules compose these; they must not
// introduce their own colors, radii, shadows, or type styles (plan.md Rule 6).
import type { LucideIcon } from "lucide-react";
import { Pause, Play, SkipBack, SkipForward } from "lucide-react";
import { useEffect, useRef, useState, type ReactNode } from "react";

import "./components.css";

export type Tone = "neutral" | "success" | "warning" | "danger" | "info";

function cx(...names: (string | false | null | undefined)[]) {
  return names.filter(Boolean).join(" ");
}

/** A rounded card surface for grouping related content. */
export function Surface({
  children,
  elevated = false,
  className,
}: {
  children: ReactNode;
  elevated?: boolean;
  className?: string;
}) {
  return <div className={cx("nd-surface", elevated && "nd-surface--elevated", className)}>{children}</div>;
}

/** Small status label. Tone communicates meaning, never decoration. */
export function Pill({ children, tone = "neutral" }: { children: ReactNode; tone?: Tone }) {
  return <span className={cx("nd-pill", `nd-pill--${tone}`)}>{children}</span>;
}

/** Marks development-only sample data so it can never be mistaken for real data (plan.md Rule 5). */
export function MockBadge() {
  return (
    <span className="nd-pill nd-pill--warning" title="Development sample data, not real">
      MOCK
    </span>
  );
}

export function IconButton({
  icon: Icon,
  label,
  onClick,
  disabled,
  active,
  size = "md",
}: {
  icon: LucideIcon;
  /** Required: icon-only buttons need an accessible name. */
  label: string;
  onClick?: () => void;
  disabled?: boolean;
  active?: boolean;
  size?: "sm" | "md" | "lg";
}) {
  return (
    <button
      type="button"
      className={cx("nd-icon-button", `nd-icon-button--${size}`, active && "nd-icon-button--active")}
      aria-label={label}
      aria-pressed={active}
      title={label}
      onClick={onClick}
      disabled={disabled}
    >
      <Icon aria-hidden size={size === "lg" ? 20 : size === "sm" ? 14 : 16} strokeWidth={2} />
    </button>
  );
}

export function Button({
  children,
  onClick,
  disabled,
  variant = "secondary",
}: {
  children: ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  variant?: "primary" | "secondary" | "warning" | "danger";
}) {
  return (
    <button
      type="button"
      className={cx("nd-button", `nd-button--${variant}`)}
      onClick={onClick}
      disabled={disabled}
    >
      {children}
    </button>
  );
}

/**
 * Two-step button for consequential actions (Git writes, approvals): the first click arms it,
 * a second click within `timeoutMs` confirms. Disarms on timeout or blur.
 */
export function ConfirmButton({
  children,
  confirmLabel,
  onConfirm,
  variant = "secondary",
  disabled,
  timeoutMs = 4000,
}: {
  children: ReactNode;
  confirmLabel: ReactNode;
  onConfirm: () => void;
  variant?: "secondary" | "warning" | "danger";
  disabled?: boolean;
  timeoutMs?: number;
}) {
  const [armed, setArmed] = useState(false);

  useEffect(() => {
    if (!armed) return;
    const timer = window.setTimeout(() => setArmed(false), timeoutMs);
    return () => window.clearTimeout(timer);
  }, [armed, timeoutMs]);

  return (
    <button
      type="button"
      className={cx(
        "nd-button",
        `nd-button--${armed ? (variant === "secondary" ? "warning" : variant) : "secondary"}`,
      )}
      aria-live="polite"
      disabled={disabled}
      onBlur={() => setArmed(false)}
      onClick={() => {
        if (armed) {
          setArmed(false);
          onConfirm();
        } else {
          setArmed(true);
        }
      }}
    >
      {armed ? confirmLabel : children}
    </button>
  );
}

/** Module surface: a subdued eyebrow title, optional trailing slot, and content. */
export function ModuleContainer({
  title,
  trailing,
  children,
}: {
  title: string;
  trailing?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="nd-module" aria-label={title}>
      <header className="nd-module__header">
        <h2 className="nd-eyebrow">{title}</h2>
        {trailing && <div className="nd-module__trailing">{trailing}</div>}
      </header>
      <div className="nd-module__body">{children}</div>
    </section>
  );
}

export function EmptyState({ title, body, action }: { title: string; body: string; action?: ReactNode }) {
  return (
    <div className="nd-state">
      <p className="nd-state__title">{title}</p>
      <p className="nd-state__body">{body}</p>
      {action && <div className="nd-state__action">{action}</div>}
    </div>
  );
}

/** Calm error surface (design.md §23): plain-language cause and a way forward, never raw errors. */
export function ErrorState({ title, body, action }: { title: string; body: string; action?: ReactNode }) {
  return (
    <div className="nd-state" role="alert">
      <p className="nd-state__title">
        <span className="nd-dot nd-dot--danger" aria-hidden /> {title}
      </p>
      <p className="nd-state__body">{body}</p>
      {action && <div className="nd-state__action">{action}</div>}
    </div>
  );
}

export function Progress({
  value,
  max = 1,
  label,
  tone = "neutral",
}: {
  value: number;
  max?: number;
  label: string;
  tone?: Tone;
}) {
  const ratio = max > 0 ? Math.min(1, Math.max(0, value / max)) : 0;
  return (
    <div
      className="nd-progress"
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={max}
      aria-valuenow={value}
    >
      <div
        className={cx("nd-progress__fill", `nd-progress__fill--${tone}`)}
        style={{ width: `${ratio * 100}%` }}
      />
    </div>
  );
}

export function NotificationCard({
  avatar,
  title,
  body,
  time,
  actions,
}: {
  /** Usually the sender's initial. */
  avatar: string;
  title: string;
  body: string;
  time: string;
  actions?: ReactNode;
}) {
  return (
    <article className="nd-notification">
      <div className="nd-avatar" aria-hidden>
        {avatar}
      </div>
      <div className="nd-notification__main">
        <div className="nd-notification__head">
          <span className="nd-notification__title">{title}</span>
          <time className="nd-notification__time">{time}</time>
        </div>
        <p className="nd-notification__body">{body}</p>
        {actions && <div className="nd-notification__actions">{actions}</div>}
      </div>
    </article>
  );
}

/** Playback controls. Unsupported controls are disabled rather than hidden or faked. */
export function MediaControl({
  playing,
  canPrevious = true,
  canNext = true,
  canPlayPause = true,
  onPrevious,
  onPlayPause,
  onNext,
}: {
  playing: boolean;
  canPrevious?: boolean;
  canNext?: boolean;
  canPlayPause?: boolean;
  onPrevious?: () => void;
  onPlayPause?: () => void;
  onNext?: () => void;
}) {
  return (
    <div className="nd-media-control" role="group" aria-label="Playback controls">
      <IconButton icon={SkipBack} label="Previous track" onClick={onPrevious} disabled={!canPrevious} />
      <IconButton
        icon={playing ? Pause : Play}
        label={playing ? "Pause" : "Play"}
        size="lg"
        onClick={onPlayPause}
        disabled={!canPlayPause}
      />
      <IconButton icon={SkipForward} label="Next track" onClick={onNext} disabled={!canNext} />
    </div>
  );
}

export interface HeatmapCell {
  /** ISO date (YYYY-MM-DD). */
  date: string;
  /** Raw value, used in the accessible label. */
  value: number;
  /** Intensity bucket 0 (none) – 4 (highest). */
  level: 0 | 1 | 2 | 3 | 4;
}

/**
 * Daily activity grid, one column per week (architecture.md §7 usage heatmap). Every cell has
 * a text label so values never depend on color alone. Scrolls horizontally when narrow.
 */
export function Heatmap({
  cells,
  label,
  unit,
}: {
  cells: HeatmapCell[];
  label: string;
  /** Noun for the value, e.g. "commits" or "tokens". */
  unit: string;
}) {
  const scroller = useRef<HTMLDivElement>(null);
  // Show the most recent weeks first when the grid overflows.
  useEffect(() => {
    const el = scroller.current;
    if (el) el.scrollLeft = el.scrollWidth;
  }, [cells]);

  return (
    <div className="nd-heatmap" ref={scroller}>
      <div className="nd-heatmap__grid" role="img" aria-label={label}>
        {cells.map((cell) => {
          const text = `${cell.date}: ${cell.value.toLocaleString()} ${unit}`;
          return (
            <span
              key={cell.date}
              className={cx("nd-heatmap__cell", `nd-heatmap__cell--l${cell.level}`)}
              title={text}
              aria-label={text}
            />
          );
        })}
      </div>
    </div>
  );
}

/** Small colored status dot for compact indicators. */
export function Dot({ tone = "neutral", label }: { tone?: Tone | "active"; label?: string }) {
  return (
    <span className={cx("nd-dot", `nd-dot--${tone}`)} role={label ? "img" : undefined} aria-label={label} />
  );
}

/** On/off control with a visible label; announces as a switch to assistive tech. */
export function Switch({
  checked,
  onChange,
  label,
  description,
  disabled,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
  description?: string;
  disabled?: boolean;
}) {
  return (
    <label className={cx("nd-switch-row", disabled && "nd-switch-row--disabled")}>
      <span className="nd-switch-row__text">
        <span className="nd-switch-row__label">{label}</span>
        {description && <span className="nd-switch-row__description">{description}</span>}
      </span>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-label={label}
        className={cx("nd-switch", checked && "nd-switch--on")}
        disabled={disabled}
        onClick={() => onChange(!checked)}
      >
        <span className="nd-switch__thumb" />
      </button>
    </label>
  );
}

/** A small set of mutually exclusive options. */
export function SegmentedControl<T extends string>({
  value,
  options,
  onChange,
  label,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
  label: string;
}) {
  return (
    <div className="nd-segmented" role="radiogroup" aria-label={label}>
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          role="radio"
          aria-checked={option.value === value}
          className={cx("nd-segmented__option", option.value === value && "nd-segmented__option--active")}
          onClick={() => onChange(option.value)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

/** Labelled single-line text input. */
export function TextField({
  label,
  value,
  onChange,
  type = "text",
  placeholder,
  description,
  disabled,
  autoComplete = "off",
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  type?: "text" | "password";
  placeholder?: string;
  description?: string;
  disabled?: boolean;
  autoComplete?: string;
}) {
  return (
    <label className="nd-field">
      <span className="nd-field__label">{label}</span>
      <input
        className="nd-field__input"
        type={type}
        value={value}
        placeholder={placeholder}
        disabled={disabled}
        autoComplete={autoComplete}
        spellCheck={false}
        onChange={(e) => onChange(e.currentTarget.value)}
      />
      {description && <span className="nd-field__description">{description}</span>}
    </label>
  );
}
