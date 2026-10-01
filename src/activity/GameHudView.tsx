// Game mode strip: FPS, GPU temperature and load, CPU, RAM, ping and the time.
import { useEffect, useState } from "react";

import { formatClock } from "../core/clock";
import type { GameHud } from "../core/generated/GameHud";
import { useSettings } from "../settings";

function Stat({ label, value, warn }: { label: string; value: string; warn?: boolean }) {
  return (
    <span className={`game-hud__stat${warn ? " game-hud__stat--warn" : ""}`}>
      <span className="game-hud__value">{value}</span>
      <span className="game-hud__label">{label}</span>
    </span>
  );
}

export function GameHudView({ hud }: { hud: GameHud }) {
  const h24 = useSettings((s) => s.settings?.home.clock24h ?? false);
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = window.setInterval(() => setNow(new Date()), 15_000);
    return () => window.clearInterval(id);
  }, []);
  const time = formatClock(now, h24);
  return (
    <div className="game-hud" role="status" aria-label={`Game mode: ${hud.app}`}>
      <Stat label="FPS" value={hud.fps != null ? String(hud.fps) : "—"} />
      {hud.gpuTemp != null && <Stat label="GPU" value={`${hud.gpuTemp}°`} warn={hud.gpuTemp >= 85} />}
      {hud.gpuLoad != null && <Stat label="load" value={`${hud.gpuLoad}%`} />}
      {hud.cpu != null && <Stat label="CPU" value={`${hud.cpu}%`} />}
      {hud.ram != null && <Stat label="RAM" value={`${hud.ram}%`} warn={hud.ram >= 90} />}
      <Stat label="ping" value={hud.pingMs != null ? `${hud.pingMs}` : "—"} warn={(hud.pingMs ?? 0) >= 120} />
      <span className="game-hud__time">{time}</span>
    </div>
  );
}
