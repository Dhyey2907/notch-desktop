import { useEffect, useState } from "react";

import { greeting, longDate } from "./dashboard";

/** Hover: a greeting for the time of day and today's date. */
export default function HomePeek() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = window.setInterval(() => setNow(new Date()), 60_000);
    return () => window.clearInterval(id);
  }, []);
  return (
    <div className="module-peek">
      <span className="module-peek__title">{greeting(now.getHours())}</span>
      <span className="module-peek__meta">{longDate(now)}</span>
    </div>
  );
}
