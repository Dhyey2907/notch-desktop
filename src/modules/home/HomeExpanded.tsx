// Home: a glance at the PC. Greeting and date with a virtual-desktop switcher; CPU, memory and
// battery rings; volume and brightness bars you can drag; screen time today; the weather.
import {
  Cloud,
  CloudDrizzle,
  CloudFog,
  CloudLightning,
  CloudMoon,
  CloudRain,
  CloudSnow,
  CloudSun,
  MapPin,
  Moon,
  RotateCw,
  Sun,
  SunDim,
  Volume1,
  Volume2,
  VolumeX,
  type LucideIcon,
} from "lucide-react";
import { motion } from "motion/react";
import { useEffect, useRef, useState, type PointerEvent, type WheelEvent } from "react";

import { eventBus } from "../../core/eventBus";
import type { Dashboard } from "../../core/generated/Dashboard";
import type { SystemStats } from "../../core/generated/SystemStats";
import type { Weather } from "../../core/generated/Weather";
import { formatTemperature } from "../../core/clock";
import { ipc } from "../../core/ipc";
import { useReduceMotion } from "../../design/useReduceMotion";
import { useSettings } from "../../settings";
import { formatMinutes, greeting, longDate } from "./dashboard";
import "./home.css";

/** How often the open dashboard refreshes CPU, memory and battery. */
const STATS_MS = 2000;
const BAR_SPRING = { type: "spring", stiffness: 380, damping: 36, mass: 0.6 } as const;
/** Drags send many levels; pass at most one this often. */
const SEND_MS = 40;

const WEATHER_ICONS: Record<string, LucideIcon> = {
  sun: Sun,
  moon: Moon,
  "cloud-sun": CloudSun,
  "cloud-moon": CloudMoon,
  cloud: Cloud,
  fog: CloudFog,
  drizzle: CloudDrizzle,
  rain: CloudRain,
  snow: CloudSnow,
  storm: CloudLightning,
};

function useDashboard() {
  const [dash, setDash] = useState<Dashboard | null>(null);
  useEffect(() => {
    let alive = true;
    ipc.systemDashboard().then(
      (d) => alive && setDash(d),
      () => {},
    );
    const patch = (change: Partial<Dashboard>) => setDash((d) => (d ? { ...d, ...change } : d));
    const offs = [
      eventBus.on("SoundChanged", ({ event }) => patch({ sound: event.payload.state })),
      eventBus.on("BrightnessChanged", ({ event }) => patch({ brightness: event.payload.level })),
      eventBus.on("ScreenTimeChanged", ({ event }) => patch({ screenTime: event.payload })),
      eventBus.on("DesktopsChanged", ({ event }) => patch({ desktops: event.payload })),
      eventBus.on("WeatherChanged", ({ event }) => patch({ weather: event.payload })),
    ];
    return () => {
      alive = false;
      offs.forEach((off) => off());
    };
  }, []);
  return dash;
}

function useStats(): SystemStats | null {
  const [stats, setStats] = useState<SystemStats | null>(null);
  useEffect(() => {
    let alive = true;
    const read = () =>
      ipc.systemStats().then(
        (s) => alive && setStats(s),
        () => {},
      );
    void read();
    const id = window.setInterval(() => void read(), STATS_MS);
    return () => {
      alive = false;
      window.clearInterval(id);
    };
  }, []);
  return stats;
}

function useNow(): Date {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = window.setInterval(() => setNow(new Date()), 60_000);
    return () => window.clearInterval(id);
  }, []);
  return now;
}

export default function HomeExpanded() {
  const dash = useDashboard();
  const stats = useStats();
  const now = useNow();
  const home = useSettings((s) => s.settings?.home);
  const show = (tile: string) => !home?.hiddenTiles.includes(tile);

  return (
    <section className="home" aria-label="Home">
      <header className="home__head">
        <div>
          <h2 className="home__greeting">{greeting(now.getHours())}</h2>
          <p className="home__date">{longDate(now)}</p>
        </div>
        {show("desktops") && dash && dash.desktops.desktops.length > 0 && <DesktopPicker dash={dash} />}
      </header>
      <div className="home__grid">
        {show("system") && <SystemCard stats={stats} />}
        {show("controls") && <ControlsCard dash={dash} />}
        {show("screen") && <ScreenTimeCard dash={dash} now={now} />}
        {show("weather") && <WeatherCard weather={dash?.weather} fahrenheit={home?.fahrenheit ?? false} />}
      </div>
    </section>
  );
}

function DesktopPicker({ dash }: { dash: Dashboard }) {
  const { desktops } = dash.desktops;
  const current = desktops.find((d) => d.current)?.index ?? 0;
  if (desktops.length < 2)
    return <span className="home-desktop home-desktop--single">{desktops[0]?.name}</span>;
  return (
    <select
      className="home-desktop"
      aria-label="Windows desktop"
      value={current}
      onChange={(e) => void ipc.desktopSwitch(Number(e.target.value)).catch(() => {})}
    >
      {desktops.map((d) => (
        <option key={d.index} value={d.index}>
          {d.name}
        </option>
      ))}
    </select>
  );
}

function Ring({ label, value, tone }: { label: string; value: number | null | undefined; tone: string }) {
  const reduce = useReduceMotion();
  const r = 20;
  const circumference = 2 * Math.PI * r;
  const shown = value ?? 0;
  return (
    <div
      className="ring"
      role="meter"
      aria-label={label}
      aria-valuenow={value ?? undefined}
      aria-valuemin={0}
      aria-valuemax={100}
    >
      <svg viewBox="0 0 48 48" className="ring__svg" aria-hidden>
        <circle cx="24" cy="24" r={r} className="ring__track" />
        <motion.circle
          cx="24"
          cy="24"
          r={r}
          className={`ring__value ring__value--${tone}`}
          strokeDasharray={circumference}
          initial={false}
          animate={{ strokeDashoffset: circumference * (1 - shown / 100) }}
          transition={reduce ? { duration: 0 } : { type: "spring", stiffness: 120, damping: 20 }}
        />
      </svg>
      <span className="ring__value-text">{value == null ? "—" : `${value}%`}</span>
      <span className="ring__label">{label}</span>
    </div>
  );
}

function SystemCard({ stats }: { stats: SystemStats | null }) {
  return (
    <div className="home-card home-card--system">
      <p className="home-card__title">System</p>
      <div className="home-rings">
        <Ring label="CPU" value={stats?.cpu} tone="cpu" />
        <Ring label="RAM" value={stats?.ram} tone="ram" />
        {stats?.battery != null ? (
          <Ring label={stats.charging ? "BAT ⚡" : "BAT"} value={stats.battery} tone="battery" />
        ) : (
          <Ring label="BAT" value={null} tone="battery" />
        )}
      </div>
    </div>
  );
}

/** Drag, click, scroll or arrow keys set a 0–1 level; the fill springs to it. */
function LevelBar({
  label,
  level,
  icon: Icon,
  onSet,
  onIcon,
  disabled,
}: {
  label: string;
  level: number;
  icon: LucideIcon;
  onSet: (level: number) => void;
  onIcon?: () => void;
  disabled?: boolean;
}) {
  const bar = useRef<HTMLDivElement>(null);
  const reduce = useReduceMotion();
  // While dragging, show the finger's level right away; otherwise follow the real level.
  const [dragging, setDragging] = useState<number | null>(null);
  const lastSent = useRef(0);
  const shown = dragging ?? level;

  const send = (value: number, force = false) => {
    const now = performance.now();
    if (force || now - lastSent.current >= SEND_MS) {
      lastSent.current = now;
      onSet(value);
    }
  };
  const at = (e: PointerEvent) => {
    const rect = bar.current!.getBoundingClientRect();
    return Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width));
  };
  const step = (delta: number) => send(Math.min(1, Math.max(0, level + delta)), true);

  return (
    <div className={`level${disabled ? " level--disabled" : ""}`}>
      <button
        type="button"
        className="level__icon"
        aria-label={label}
        title={label}
        onClick={onIcon}
        disabled={!onIcon}
      >
        <Icon size={15} aria-hidden />
      </button>
      <div
        ref={bar}
        className="level__bar"
        role="slider"
        aria-label={label}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(shown * 100)}
        aria-disabled={disabled}
        tabIndex={disabled ? -1 : 0}
        onPointerDown={(e) => {
          if (disabled) return;
          e.currentTarget.setPointerCapture(e.pointerId);
          const v = at(e);
          setDragging(v);
          send(v, true);
        }}
        onPointerMove={(e) => {
          if (dragging === null) return;
          const v = at(e);
          setDragging(v);
          send(v);
        }}
        onPointerUp={(e) => {
          if (dragging === null) return;
          send(at(e), true);
          setDragging(null);
        }}
        onWheel={(e: WheelEvent) => !disabled && step(e.deltaY < 0 ? 0.02 : -0.02)}
        onKeyDown={(e) => {
          if (disabled) return;
          if (e.key === "ArrowRight" || e.key === "ArrowUp") step(0.05);
          if (e.key === "ArrowLeft" || e.key === "ArrowDown") step(-0.05);
        }}
      >
        <motion.span
          className="level__fill"
          initial={false}
          animate={{ width: `${shown * 100}%` }}
          transition={dragging !== null || reduce ? { duration: 0 } : BAR_SPRING}
        />
      </div>
      <span className="level__value">{disabled ? "—" : Math.round(shown * 100)}</span>
    </div>
  );
}

function ControlsCard({ dash }: { dash: Dashboard | null }) {
  const sound = dash?.sound;
  const brightness = dash?.brightness;
  const volumeIcon =
    !sound || sound.muted || sound.level === 0 ? VolumeX : sound.level < 0.5 ? Volume1 : Volume2;
  return (
    <div className="home-card home-card--controls">
      <p className="home-card__title">
        Sound &amp; display
        {sound?.device && <span className="home-card__hint">{sound.device}</span>}
      </p>
      <LevelBar
        label={sound?.muted ? "Unmute" : "Volume"}
        level={sound?.muted ? 0 : (sound?.level ?? 0)}
        icon={volumeIcon}
        disabled={!sound}
        onSet={(v) => void ipc.soundSetVolume(v).catch(() => {})}
        onIcon={sound ? () => void ipc.soundToggleMute().catch(() => {}) : undefined}
      />
      <LevelBar
        label="Brightness"
        level={(brightness ?? 0) / 100}
        icon={brightness != null && brightness < 40 ? SunDim : Sun}
        disabled={brightness == null}
        onSet={(v) => void ipc.brightnessSet(Math.round(v * 100)).catch(() => {})}
      />
    </div>
  );
}

function ScreenTimeCard({ dash, now }: { dash: Dashboard | null; now: Date }) {
  const time = dash?.screenTime;
  const hour = now.getHours();
  const peak = Math.max(30, ...(time?.hours ?? [0]));
  return (
    <div className="home-card home-card--screen">
      <div>
        <p className="home-card__title">Screen time</p>
        <p className="home-card__big">{time ? formatMinutes(time.minutes) : "—"}</p>
        <p className="home-card__sub">Tracked today</p>
      </div>
      <div className="home-bars" aria-hidden>
        {(time?.hours ?? Array(24).fill(0)).map((m: number, h: number) => (
          <motion.span
            key={h}
            className={`home-bars__bar${h === hour ? " home-bars__bar--now" : ""}${h > hour ? " home-bars__bar--later" : ""}`}
            initial={false}
            animate={{ height: `${Math.max(6, (m / peak) * 100)}%` }}
            transition={{ type: "spring", stiffness: 200, damping: 26 }}
          />
        ))}
      </div>
    </div>
  );
}

function WeatherCard({ weather, fahrenheit }: { weather: Weather | undefined; fahrenheit: boolean }) {
  const Icon = WEATHER_ICONS[weather?.icon ?? "cloud"] ?? Cloud;
  const status = weather?.status ?? "loading";
  return (
    <div className="home-card home-card--weather">
      <div className="home-weather__text">
        <p className="home-card__title">Weather{weather?.place ? ` · ${weather.place}` : ""}</p>
        {status === "ready" ? (
          <>
            <p className="home-card__big">
              {weather!.temperature != null ? formatTemperature(weather!.temperature, fahrenheit) : "—"}
            </p>
            <p className="home-card__sub">{weather!.condition}</p>
          </>
        ) : status === "noAccess" ? (
          <>
            <p className="home-card__sub">Location is off for desktop apps.</p>
            <button
              type="button"
              className="home-link"
              onClick={() => void ipc.openLocationSettings().catch(() => {})}
            >
              <MapPin size={12} aria-hidden /> Turn on location
            </button>
          </>
        ) : status === "unavailable" ? (
          <>
            <p className="home-card__sub">Weather isn't available right now.</p>
            <button
              type="button"
              className="home-link"
              onClick={() => void ipc.weatherRefresh().catch(() => {})}
            >
              <RotateCw size={12} aria-hidden /> Try again
            </button>
          </>
        ) : (
          <p className="home-card__sub">Finding the weather…</p>
        )}
      </div>
      {status === "ready" && <Icon className="home-weather__icon" size={34} aria-hidden />}
    </div>
  );
}
