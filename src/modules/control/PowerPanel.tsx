// Control → Power: lock, sleep, hibernate, restart (or into BIOS), shut down, sign out. Everything
// but Lock asks once more, since it closes your apps.
import {
  ArrowLeft,
  Cpu,
  Lock,
  LogOut,
  Moon,
  Power,
  RotateCcw,
  Snowflake,
  type LucideIcon,
} from "lucide-react";
import { useState } from "react";

import type { PowerAction } from "../../core/generated/PowerAction";
import { ipc, isIpcError } from "../../core/ipc";
import { Button, IconButton } from "../../design/components";

type Choice = PowerAction | "lock";

const CHOICES: { id: Choice; label: string; icon: LucideIcon; warn?: string }[] = [
  { id: "lock", label: "Lock", icon: Lock },
  { id: "sleep", label: "Sleep", icon: Moon, warn: "The PC goes to sleep." },
  { id: "hibernate", label: "Hibernate", icon: Snowflake, warn: "The PC saves everything and powers off." },
  { id: "restart", label: "Restart", icon: RotateCcw, warn: "Unsaved work in open apps may be lost." },
  {
    id: "restartBios",
    label: "Restart to BIOS",
    icon: Cpu,
    warn: "Windows asks for admin, then restarts into the firmware setup.",
  },
  { id: "shutdown", label: "Shut down", icon: Power, warn: "Unsaved work in open apps may be lost." },
  { id: "signOut", label: "Sign out", icon: LogOut, warn: "All your apps close." },
];

/** Run a power choice (Lock straight away; the rest after the UI has confirmed). */
function runPower(choice: Choice): Promise<void> {
  return choice === "lock" ? ipc.systemLock() : ipc.systemPower(choice);
}

export function PowerPanel({ onBack }: { onBack: () => void }) {
  const [asking, setAsking] = useState<(typeof CHOICES)[number] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const go = (choice: Choice) =>
    void runPower(choice).then(
      () => setAsking(null),
      (e) => setError(isIpcError(e) ? e.message : "That didn't work"),
    );

  return (
    <section className="control network" aria-label="Power">
      <div className="network__bar">
        <IconButton icon={ArrowLeft} label="Back" size="sm" onClick={onBack} />
        <span className="network__heading">Power</span>
      </div>
      {asking ? (
        <div className="power-confirm" role="alertdialog" aria-label={`${asking.label}?`}>
          <p className="power-confirm__title">{asking.label} now?</p>
          <p className="control-card__sub">{asking.warn}</p>
          <div className="power-confirm__row">
            <Button onClick={() => setAsking(null)}>Cancel</Button>
            <Button variant="danger" onClick={() => go(asking.id)}>
              {asking.label}
            </Button>
          </div>
        </div>
      ) : (
        <div className="power-grid">
          {CHOICES.map((c) => (
            <button
              key={c.id}
              type="button"
              className="power-grid__item"
              onClick={() => (c.warn ? setAsking(c) : go(c.id))}
            >
              <c.icon size={16} aria-hidden />
              <span>{c.label}</span>
            </button>
          ))}
        </div>
      )}
      {error && (
        <p className="control-error" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
