import { useEffect, useState } from "react";

import type { NetSpeed } from "../../core/generated/NetSpeed";
import { ipc } from "../../core/ipc";

/** Live network speed while the panel is open (one reading a second). */
export function useSpeed(): NetSpeed | null {
  const [speed, setSpeed] = useState<NetSpeed | null>(null);
  useEffect(() => {
    let alive = true;
    // The first call only sets the baseline.
    void ipc.controlSpeed().catch(() => {});
    const id = window.setInterval(
      () =>
        ipc.controlSpeed().then(
          (s) => alive && setSpeed(s),
          () => {},
        ),
      1000,
    );
    return () => {
      alive = false;
      window.clearInterval(id);
    };
  }, []);
  return speed;
}
