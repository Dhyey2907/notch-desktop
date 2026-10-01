import { useEffect, useState } from "react";

import { formatClock } from "../core/clock";
import { useSettings } from "../settings";

/** The time in the resting bubble (Settings → Look). */
export function BubbleClock() {
  const h24 = useSettings((s) => s.settings?.home.clock24h ?? false);
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = window.setInterval(() => setNow(new Date()), 10_000);
    return () => window.clearInterval(id);
  }, []);
  return <span className="island-clock">{formatClock(now, h24, false)}</span>;
}
