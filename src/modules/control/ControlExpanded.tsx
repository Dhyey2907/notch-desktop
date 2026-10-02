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
  Moon,
  Plane,
  Radio,
  ShieldCheck,
  Wifi,
  WifiOff,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useState } from "react";

import type { ControlSnapshot } from "../../core/generated/ControlSnapshot";
import type { RadioKind } from "../../core/generated/RadioKind";
import { ipc, isIpcError } from "../../core/ipc";
import { useControl } from "./controlStore";
import { formatMinutes, formatSpeed } from "./format";
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
  return (
    <div className="control-toggles" role="group" aria-label="Quick switches">
      <Toggle icon={Wifi} label="Wi-Fi" on={radios.wifi} onClick={() => void flip("wifi", radios.wifi)} />
      <Toggle
        icon={Bluetooth}
        label="Bluetooth"
        on={radios.bluetooth}
        onClick={() => void flip("bluetooth", radios.bluetooth)}
      />
      <Toggle
        icon={Radio}
        label="Hotspot"
        on={radios.hotspot}
        onClick={() => void flip("hotspot", radios.hotspot)}
      />
      <Toggle icon={Plane} label="Airplane" external onClick={() => open("airplane")} />
      <Toggle icon={Moon} label="Night light" external onClick={() => open("nightlight")} />
    </div>
  );
}

function NetworkCard({ snapshot }: { snapshot: ControlSnapshot }) {
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
  // Fresh readings each time the panel opens (headphones may have just connected).
  useEffect(() => {
    ipc.controlRefresh().then(
      (s) => useControl.getState().set(s),
      () => {},
    );
  }, []);
  if (!snapshot) return <section className="control" aria-label="Control" />;
  return (
    <section className="control" aria-label="Control">
      <Toggles snapshot={snapshot} onError={setError} />
      {error && (
        <p className="control-error" role="alert">
          {error}
        </p>
      )}
      <div className="control-cards">
        <NetworkCard snapshot={snapshot} />
        <PowerCard snapshot={snapshot} />
      </div>
      <Devices snapshot={snapshot} />
    </section>
  );
}
