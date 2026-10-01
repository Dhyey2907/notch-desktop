import { useCallback, useEffect, useState } from "react";

import type { TotpCode } from "../../core/generated/TotpCode";
import { ipc } from "../../core/ipc";

/** 2FA codes, refreshed every second while shown. */
export function useCodes() {
  const [codes, setCodes] = useState<TotpCode[] | null>(null);
  const refresh = useCallback(
    () =>
      ipc.totpCodes().then(
        (c) => setCodes(c),
        () => setCodes([]),
      ),
    [],
  );
  useEffect(() => {
    void refresh();
    const id = window.setInterval(() => void refresh(), 1000);
    return () => window.clearInterval(id);
  }, [refresh]);
  return { codes, refresh };
}
