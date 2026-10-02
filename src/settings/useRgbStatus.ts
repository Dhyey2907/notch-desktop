import { useEffect, useState } from "react";

import type { RgbStatus } from "../core/generated/RgbStatus";
import { ipc } from "../core/ipc";

/** OpenRGB's connection, checked every two seconds while the lights setting is on. */
export function useRgbStatus(on: boolean): RgbStatus | null {
  const [status, setStatus] = useState<RgbStatus | null>(null);
  useEffect(() => {
    if (!on) return;
    let alive = true;
    const check = () =>
      ipc.rgbStatus().then(
        (s) => alive && setStatus(s),
        () => {},
      );
    void check();
    const id = window.setInterval(() => void check(), 2000);
    return () => {
      alive = false;
      window.clearInterval(id);
    };
  }, [on]);
  return on ? status : null;
}
