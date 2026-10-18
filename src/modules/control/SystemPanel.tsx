// Control → System: CPU, memory and GPU load, the apps using the most of each, and how healthy
// and full the drives are.
import { ArrowLeft, HardDrive } from "lucide-react";
import { useEffect, useState } from "react";

import type { DiskHealth } from "../../core/generated/DiskHealth";
import type { SystemMonitor } from "../../core/generated/SystemMonitor";
import { ipc, isIpcError } from "../../core/ipc";
import { IconButton, Progress } from "../../design/components";

const REFRESH_MS = 3000;

function Meter({ label, value, extra }: { label: string; value?: number; extra?: string }) {
  return (
    <div className="sysmon__meter">
      <span className="sysmon__label">{label}</span>
      <Progress
        value={value ?? 0}
        max={100}
        label={`${label} use`}
        tone={(value ?? 0) >= 90 ? "warning" : "neutral"}
      />
      <span className="sysmon__value">
        {value === undefined ? "–" : `${value}%`}
        {extra ? ` · ${extra}` : ""}
      </span>
    </div>
  );
}

export function SystemPanel({ onBack }: { onBack: () => void }) {
  const [mon, setMon] = useState<SystemMonitor | null>(null);
  const [disks, setDisks] = useState<DiskHealth | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    const read = () =>
      ipc.systemMonitor().then(
        (m) => live && setMon(m),
        (e) => live && setError(isIpcError(e) ? e.message : "Couldn't read the system"),
      );
    void read();
    const id = window.setInterval(() => void read(), REFRESH_MS);
    ipc.diskHealth(false).then(
      (d) => live && setDisks(d),
      () => {},
    );
    return () => {
      live = false;
      window.clearInterval(id);
    };
  }, []);

  return (
    <section className="control network" aria-label="System">
      <div className="network__bar">
        <IconButton icon={ArrowLeft} label="Back" size="sm" onClick={onBack} />
        <span className="network__heading">System</span>
      </div>
      <div className="sysmon">
        <Meter label="CPU" value={mon?.cpu} />
        <Meter label="Memory" value={mon?.ram} />
        {mon?.gpu !== undefined && (
          <Meter
            label="GPU"
            value={mon.gpu}
            extra={mon.gpuTemp !== undefined ? `${mon.gpuTemp}°C` : undefined}
          />
        )}
        <div className="sysmon__lists">
          <div>
            <p className="sysmon__heading">Busiest</p>
            <ul className="sysmon__apps">
              {(mon?.byCpu ?? []).map((a) => (
                <li key={a.exe} title={a.exe}>
                  <span>{a.name}</span>
                  <span>{a.cpu === undefined ? "…" : `${a.cpu}%`}</span>
                </li>
              ))}
            </ul>
          </div>
          <div>
            <p className="sysmon__heading">Most memory</p>
            <ul className="sysmon__apps">
              {(mon?.byRam ?? []).map((a) => (
                <li key={a.exe} title={a.exe}>
                  <span>{a.name}</span>
                  <span>{a.ramMb >= 1024 ? `${(a.ramMb / 1024).toFixed(1)} GB` : `${a.ramMb} MB`}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>
        {disks && (
          <div className="sysmon__disks">
            {disks.problem && (
              <p className="control-error" role="alert">
                {disks.problem}
              </p>
            )}
            {disks.volumes.map((v) => (
              <div key={v.letter} className="sysmon__meter">
                <span className="sysmon__label">
                  <HardDrive size={12} aria-hidden /> {v.letter}:
                </span>
                <Progress
                  value={v.usedPercent}
                  max={100}
                  label={`Drive ${v.letter} used`}
                  tone={v.usedPercent >= 90 ? "warning" : "neutral"}
                />
                <span className="sysmon__value">{v.freeGb} GB free</span>
              </div>
            ))}
            <p className="control-card__sub">
              {disks.drives
                .map((d) => `${d.name || "Drive"}${d.kind ? ` (${d.kind})` : ""}: ${d.health}`)
                .join(" · ")}
            </p>
          </div>
        )}
      </div>
      {error && <p className="control-error">{error}</p>}
    </section>
  );
}
