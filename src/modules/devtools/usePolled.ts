import { useEffect, useState } from "react";

/** Load something now and again every `ms` while shown (errors leave the last value). */
export function usePolled<T>(load: () => Promise<T>, ms: number): T | null {
  const [value, setValue] = useState<T | null>(null);
  useEffect(() => {
    let alive = true;
    const read = () =>
      load().then(
        (v) => alive && setValue(v),
        () => {},
      );
    void read();
    const id = window.setInterval(() => void read(), ms);
    return () => {
      alive = false;
      window.clearInterval(id);
    };
    // `load` is a stable ipc call per caller.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ms]);
  return value;
}
