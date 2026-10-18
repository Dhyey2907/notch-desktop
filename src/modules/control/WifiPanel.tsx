// Control → Wi-Fi: networks nearby with their signal. One click joins a network this PC has
// joined before; a new one opens Windows' own Wi-Fi list (Overline never handles passwords).
import { ArrowLeft, Check, ExternalLink, Lock, RefreshCw } from "lucide-react";
import { useCallback, useEffect, useState } from "react";

import type { WifiNetwork } from "../../core/generated/WifiNetwork";
import { ipc, isIpcError } from "../../core/ipc";
import { IconButton } from "../../design/components";

function Bars({ signal }: { signal: number }) {
  const n = signal >= 85 ? 4 : signal >= 65 ? 3 : signal >= 40 ? 2 : signal >= 15 ? 1 : 0;
  return (
    <span className="wifi__bars" aria-label={`Signal ${signal}%`}>
      {[1, 2, 3, 4].map((i) => (
        <span
          key={i}
          className={i <= n ? "wifi__bar wifi__bar--on" : "wifi__bar"}
          style={{ height: `${i * 3 + 2}px` }}
        />
      ))}
    </span>
  );
}

export function WifiPanel({ onBack }: { onBack: () => void }) {
  const [list, setList] = useState<WifiNetwork[] | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    ipc.wifiNetworks().then(setList, (e) => {
      setList([]);
      setError(isIpcError(e) ? e.message : "Couldn't list Wi-Fi networks");
    });
  }, []);

  useEffect(() => {
    // Ask for a fresh scan; results land a few seconds later.
    void ipc.wifiScan().catch(() => {});
    load();
    const id = window.setTimeout(load, 4000);
    return () => window.clearTimeout(id);
  }, [load]);

  const join = (n: WifiNetwork) => {
    if (!n.known) {
      void ipc.wifiOpenSettings().catch(() => {});
      return;
    }
    setBusy(n.ssid);
    setError(null);
    void ipc
      .wifiConnect(n.ssid)
      .then(
        () => window.setTimeout(load, 3000),
        (e) => setError(isIpcError(e) ? e.message : "Couldn't connect"),
      )
      .finally(() => window.setTimeout(() => setBusy(null), 3000));
  };

  return (
    <section className="control network" aria-label="Wi-Fi">
      <div className="network__bar">
        <IconButton icon={ArrowLeft} label="Back" size="sm" onClick={onBack} />
        <span className="network__heading">Wi-Fi</span>
        <IconButton
          icon={RefreshCw}
          label="Look again"
          size="sm"
          onClick={() => {
            void ipc.wifiScan().catch(() => {});
            window.setTimeout(load, 3000);
          }}
        />
      </div>
      {error && <p className="control-error">{error}</p>}
      <ul className="wifi">
        {(list ?? []).map((n) => (
          <li key={n.ssid}>
            <button
              type="button"
              className={`wifi__item${n.connected ? " wifi__item--on" : ""}`}
              disabled={n.connected || busy !== null}
              title={
                n.known ? `Join ${n.ssid}` : "New network: opens Windows' Wi-Fi list to enter its password"
              }
              onClick={() => join(n)}
            >
              <Bars signal={n.signal} />
              <span className="wifi__name">{n.ssid}</span>
              {n.secure && <Lock size={11} aria-label="Secured" />}
              <span className="wifi__state">
                {n.connected ? (
                  <>
                    <Check size={12} aria-hidden /> Connected
                  </>
                ) : busy === n.ssid ? (
                  "Joining…"
                ) : n.known ? (
                  "Join"
                ) : (
                  <>
                    New <ExternalLink size={11} aria-hidden />
                  </>
                )}
              </span>
            </button>
          </li>
        ))}
        {list && list.length === 0 && !error && <li className="control-card__sub">No networks found.</li>}
      </ul>
    </section>
  );
}
