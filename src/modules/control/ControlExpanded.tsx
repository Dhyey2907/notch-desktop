// Control center: quick switches on top, then the network and the battery, then accessories.
import {
  ArrowDown,
  ArrowUp,
  BatteryCharging,
  BatteryFull,
  BatteryLow,
  BatteryMedium,
  Bluetooth,
  Cable,
  ExternalLink,
  Gamepad2,
  Headphones,
  Plane,
  Radio,
  ShieldCheck,
  Wifi,
  WifiOff,
  type LucideIcon,
} from "lucide-react";
import { Fragment, useEffect, useState, type ReactNode } from "react";

import type { ControlSnapshot } from "../../core/generated/ControlSnapshot";
import type { RadioKind } from "../../core/generated/RadioKind";
import { ipc, isIpcError } from "../../core/ipc";
import { useArranged } from "../../shell/layout";
import { useControl } from "./controlStore";
import { SECTIONS, TOGGLES } from "./layout";
import { formatMinutes, formatSpeed } from "./format";
import { NetworkPanel } from "./NetworkPanel";
import { DisplayRow } from "./DisplayRow";
import { PowerPanel } from "./PowerPanel";
import { WifiPanel } from "./WifiPanel";
import { SystemPanel } from "./SystemPanel";
import { SoundPanel } from "./SoundCard";
import { useSpeed } from "./useSpeed";
import "./control.css";

function Toggle({
  icon: Icon,
  label,
  on,
  external,
  onClick,
}: {
  icon: LucideIcon;
  label: string;
  /** `undefined`: this PC doesn't have it. */
  on?: boolean;
  /** Opens Windows Settings instead of switching here. */
  external?: boolean;
  onClick: () => void;
}) {
  const missing = on === undefined && !external;
  return (
    <button
      type="button"
      className={`control-toggle${on ? " control-toggle--on" : ""}`}
      aria-pressed={external ? undefined : on}
      aria-label={external ? `${label} (opens Windows Settings)` : label}
      title={missing ? `${label} isn't available on this PC` : external ? "Opens Windows Settings" : label}
      disabled={missing}
      onClick={onClick}
    >
      <span className="control-toggle__circle">
        <Icon size={17} aria-hidden />
        {external && <ExternalLink size={9} className="control-toggle__external" aria-hidden />}
      </span>
      <span className="control-toggle__label">{label}</span>
    </button>
  );
}

function Toggles({
  snapshot,
  onError,
}: {
  snapshot: ControlSnapshot;
  onError: (text: string | null) => void;
}) {
  const { radios } = snapshot;
  const [busy, setBusy] = useState<RadioKind | null>(null);
  const flip = async (kind: RadioKind, on: boolean | undefined) => {
    if (on === undefined || busy) return;
    setBusy(kind);
    onError(null);
    try {
      await ipc.controlSetRadio(kind, !on);
    } catch (e) {
      onError(isIpcError(e) ? e.message.replace(/^invalid input: /, "") : "That didn't work.");
    } finally {
      setBusy(null);
    }
  };
  const open = (page: string) => void ipc.controlOpenSettings(page).catch(() => {});
  const shown = useArranged("control.toggles", TOGGLES);
  const toggle = (id: string) => {
    if (id === "wifi")
      return (
        <Toggle icon={Wifi} label="Wi-Fi" on={radios.wifi} onClick={() => void flip("wifi", radios.wifi)} />
      );
    if (id === "bluetooth")
      return (
        <Toggle
          icon={Bluetooth}
          label="Bluetooth"
          on={radios.bluetooth}
          onClick={() => void flip("bluetooth", radios.bluetooth)}
        />
      );
    if (id === "hotspot")
      return (
        <Toggle
          icon={Radio}
          label="Hotspot"
          on={radios.hotspot}
          onClick={() => void flip("hotspot", radios.hotspot)}
        />
      );
    return <Toggle icon={Plane} label="Airplane" external onClick={() => open("airplane")} />;
  };
  if (shown.length === 0) return null;
  return (
    <div className="control-toggles" role="group" aria-label="Quick switches">
      {shown.map(({ id }) => (
        <Fragment key={id}>{toggle(id)}</Fragment>
      ))}
    </div>
  );
}

function NetworkCard({ snapshot, onMore }: { snapshot: ControlSnapshot; onMore: () => void }) {
  const speed = useSpeed();
  const { network } = snapshot;
  const Icon = network.internet === "offline" ? WifiOff : network.kind === "ethernet" ? Cable : Wifi;
  const name =
    network.ssid ??
    (network.kind === "wifi"
      ? "Wi-Fi"
      : network.kind === "ethernet"
        ? "Ethernet"
        : network.kind === "cellular"
          ? "Mobile data"
          : "Network");
  const status =
    network.internet === "online"
      ? "Connected"
      : network.internet === "limited"
        ? "No internet access"
        : "Offline";
  return (
    <div className="control-card">
      <div className="control-card__head">
        <Icon size={15} aria-hidden />
        <span className="control-card__name" title={name}>
          {network.internet === "offline" ? "Not connected" : name}
        </span>
        {network.bars != null && (
          <span className="control-bars" role="img" aria-label={`Signal ${network.bars} of 5`}>
            {[1, 2, 3, 4, 5].map((b) => (
              <span key={b} className={b <= network.bars! ? "control-bars__on" : undefined} />
            ))}
          </span>
        )}
      </div>
      <p className={`control-card__sub${network.internet !== "online" ? " control-card__sub--warn" : ""}`}>
        {status}
        {network.vpn && (
          <span className="control-pill">
            <ShieldCheck size={11} aria-hidden /> {network.vpn}
          </span>
        )}
      </p>
      <div className="control-speeds" aria-label="Network speed">
        <span>
          <ArrowDown size={12} aria-hidden /> {speed ? formatSpeed(speed.down) : "…"}
        </span>
        <span>
          <ArrowUp size={12} aria-hidden /> {speed ? formatSpeed(speed.up) : "…"}
        </span>
      </div>
      <button type="button" className="nd-chip control-card__more" onClick={onMore}>
        Speed test & apps
      </button>
    </div>
  );
}

function PowerCard({ snapshot }: { snapshot: ControlSnapshot }) {
  const { power } = snapshot;
  if (!power) {
    return (
      <div className="control-card">
        <div className="control-card__head">
          <Cable size={15} aria-hidden />
          <span className="control-card__name">Plugged in</span>
        </div>
        <p className="control-card__sub">This PC has no battery.</p>
      </div>
    );
  }
  const Icon = power.charging
    ? BatteryCharging
    : power.percent <= 20
      ? BatteryLow
      : power.percent < 70
        ? BatteryMedium
        : BatteryFull;
  const status = power.charging
    ? power.minutes != null
      ? `Charging · full in ${formatMinutes(power.minutes)}`
      : "Charging"
    : power.plugged
      ? "Plugged in, not charging"
      : power.minutes != null
        ? `${formatMinutes(power.minutes)} left`
        : "On battery";
  return (
    <div className="control-card">
      <div className="control-card__head">
        <Icon
          size={15}
          aria-hidden
          className={power.percent <= 20 && !power.plugged ? "control-low" : undefined}
        />
        <span className="control-card__name">{power.percent}%</span>
      </div>
      <p className="control-card__sub">{status}</p>
      <p className="control-card__meta">
        {power.health != null && <>Health {power.health}%</>}
        {power.health != null && power.cycles != null && " · "}
        {power.cycles != null && <>{power.cycles} cycles</>}
        {power.health == null && power.cycles == null && (
          <button type="button" className="home-link" onClick={() => void ipc.controlOpenSettings("battery")}>
            Battery settings
          </button>
        )}
      </p>
    </div>
  );
}

function Devices({ snapshot }: { snapshot: ControlSnapshot }) {
  if (snapshot.devices.length === 0) {
    return <p className="control-devices__empty">No accessories reporting their battery right now.</p>;
  }
  return (
    <ul className="control-devices" aria-label="Accessory batteries">
      {snapshot.devices.map((d) => {
        const Icon = d.kind === "controller" ? Gamepad2 : Headphones;
        return (
          <li key={`${d.kind}-${d.name}`} className="control-device">
            <Icon size={13} aria-hidden />
            <span className="control-device__name" title={d.name}>
              {d.name}
            </span>
            <span className="control-device__bar" aria-hidden>
              <span
                className={d.percent <= 15 ? "control-low-fill" : undefined}
                style={{ width: `${d.percent}%` }}
              />
            </span>
            <span className="control-device__value">{d.percent}%</span>
          </li>
        );
      })}
    </ul>
  );
}

export default function ControlExpanded() {
  const snapshot = useControl((s) => s.snapshot);
  const [error, setError] = useState<string | null>(null);
  const [network, setNetwork] = useState(false);
  const [sound, setSound] = useState(false);
  const [power, setPower] = useState(false);
  const [wifi, setWifi] = useState(false);
  const [system, setSystem] = useState(false);
  const sections = useArranged("control", SECTIONS);
  // Fresh readings each time the panel opens (headphones may have just connected).
  useEffect(() => {
    ipc.controlRefresh().then(
      (s) => useControl.getState().set(s),
      () => {},
    );
  }, []);
  if (!snapshot) return <section className="control" aria-label="Control" />;
  if (network) return <NetworkPanel onBack={() => setNetwork(false)} />;
  if (sound) return <SoundPanel onBack={() => setSound(false)} />;
  if (power) return <PowerPanel onBack={() => setPower(false)} />;
  if (wifi) return <WifiPanel onBack={() => setWifi(false)} />;
  if (system) return <SystemPanel onBack={() => setSystem(false)} />;
  const section = (id: string): ReactNode => {
    if (id === "toggles") return <Toggles snapshot={snapshot} onError={setError} />;
    if (id === "display") return <DisplayRow onError={setError} />;
    if (id === "network") return <NetworkCard snapshot={snapshot} onMore={() => setNetwork(true)} />;
    if (id === "battery") return <PowerCard snapshot={snapshot} />;
    if (id === "devices") return <Devices snapshot={snapshot} />;
    return more;
  };
  // The network and battery cards sit side by side when they are next to each other.
  const rows: string[][] = [];
  for (const { id } of sections) {
    const last = rows[rows.length - 1];
    const card = id === "network" || id === "battery";
    if (card && last && last.length === 1 && (last[0] === "network" || last[0] === "battery")) last.push(id);
    else rows.push([id]);
  }
  const more = (
    <div className="control-more">
      <button type="button" className="nd-chip" onClick={() => setSound(true)}>
        Sound · apps &amp; output
      </button>
      <button type="button" className="nd-chip" onClick={() => setWifi(true)}>
        Wi-Fi networks
      </button>
      <button type="button" className="nd-chip" onClick={() => setSystem(true)}>
        System
      </button>
      <button type="button" className="nd-chip" onClick={() => setPower(true)}>
        Power
      </button>
    </div>
  );
  return (
    <section className="control" aria-label="Control">
      {error && (
        <p className="control-error" role="alert">
          {error}
        </p>
      )}
      {rows.map((row) =>
        row[0] === "network" || row[0] === "battery" ? (
          <div key={row.join("+")} className="control-cards">
            {row.map((id) => (
              <Fragment key={id}>{section(id)}</Fragment>
            ))}
          </div>
        ) : (
          <Fragment key={row[0]}>{section(row[0])}</Fragment>
        ),
      )}
    </section>
  );
}
