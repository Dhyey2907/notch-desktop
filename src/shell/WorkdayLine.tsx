import { useEffect, useState } from "react";

import { workday } from "../core/workday";
import { useSettings } from "../settings";

/** A hairline under the resting bubble showing how much of the workday has passed. */
export function WorkdayLine() {
  const day = useSettings((s) => s.settings?.day);
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = window.setInterval(() => setNow(new Date()), 60_000);
    return () => window.clearInterval(id);
  }, []);
  if (!day || day.noWorkdayBar) return null;
  const w = workday(day, now);
  if (w.phase !== "during") return null;
  return (
    <span className="island-workday" aria-hidden>
      <span style={{ width: `${Math.round(w.fraction * 100)}%` }} />
    </span>
  );
}
