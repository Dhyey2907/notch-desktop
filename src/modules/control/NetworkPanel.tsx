// Network details: a speed test (speed.cloudflare.com) and which apps are using the internet
// right now, with a rough rate each.
import { ArrowDown, ArrowLeft, ArrowUp, Gauge, Timer } from "lucide-react";
import { useEffect, useState } from "react";

import type { AppTraffic } from "../../core/generated/AppTraffic";
import type { SpeedTest } from "../../core/generated/SpeedTest";
import { eventBus } from "../../core/eventBus";
import { ipc, isIpcError } from "../../core/ipc";
import { Button, IconButton } from "../../design/components";
import { useMinuteClock } from "../clipboard/useMinuteClock";
import { formatSpeed, speedTestLine } from "./format";

const APPS_EVERY_MS = 2000;

function useNetApps(): AppTraffic[] | null {
  const [apps, setApps] = useState<AppTraffic[] | null>(null);
  useEffect(() => {
    let alive = true;
    const read = () =>
      ipc.controlNetApps().then(
        (a) => alive && setApps(a),
        () => {},
      );
    void read();
    const id = window.setInterval(read, APPS_EVERY_MS);
    return () => {
      alive = false;
      window.clearInterval(id);
    };
  }, []);
  return apps;
}

function useSpeedTest(): SpeedTest | null {
  const [test, setTest] = useState<SpeedTest | null>(null);
  useEffect(() => {
    ipc.controlSpeedTestLast().then(
      (t) => setTest((current) => current ?? t),
      () => {},
    );
    return eventBus.on("SpeedTestChanged", ({ event }) => setTest(event.payload));
  }, []);
  return test;
}

export function NetworkPanel({ onBack }: { onBack: () => void }) {
  const apps = useNetApps();
  const test = useSpeedTest();
  const [error, setError] = useState<string | null>(null);
  const now = useMinuteClock();
  const running = test?.phase === "ping" || test?.phase === "download" || test?.phase === "upload";
  const busiest = Math.max(1, ...(apps ?? []).map((a) => a.rate));

  const run = () => {
    setError(null);
    ipc
      .controlSpeedTest()
      .catch((e) =>
        setError(isIpcError(e) ? e.message.replace(/^invalid input: /, "") : "The speed test failed"),
      );
  };

  return (
    <section className="control network" aria-label="Network">
      <div className="network__bar">
        <IconButton icon={ArrowLeft} label="Back" size="sm" onClick={onBack} />
        <span className="network__heading">Network</span>
      </div>

      <div className="control-card network__test">
        <div className="control-card__head">
          <Gauge size={15} aria-hidden />
          <span className="control-card__name">Speed test</span>
          <Button variant={running ? "secondary" : "primary"} disabled={running} onClick={run}>
            {running ? "Testing…" : test ? "Test again" : "Run test"}
          </Button>
        </div>
        <div className="network__results" aria-live="polite">
          <Result
            icon={Timer}
            label="Ping"
            value={test?.pingMs != null ? `${test.pingMs} ms` : null}
            busy={test?.phase === "ping"}
          />
          <Result
            icon={ArrowDown}
            label="Download"
            value={test?.downMbps != null ? `${test.downMbps} Mbps` : null}
            busy={test?.phase === "download"}
          />
          <Result
            icon={ArrowUp}
            label="Upload"
            value={test?.upMbps != null ? `${test.upMbps} Mbps` : null}
            busy={test?.phase === "upload"}
          />
        </div>
        <p className="control-card__sub">
          {error ?? (test ? speedTestLine(test, now) : "About 20 seconds, using speed.cloudflare.com.")}
        </p>
      </div>

      <div className="control-card network__apps">
        <div className="control-card__head">
          <span className="control-card__name">Using the internet now</span>
        </div>
        {apps === null ? (
          <p className="control-card__sub">Looking…</p>
        ) : apps.length === 0 ? (
          <p className="control-card__sub">No apps have open connections.</p>
        ) : (
          <ul className="network__list">
            {apps.map((a) => (
              <li key={a.name} className="network__app">
                <span className="network__app-name" title={a.name}>
                  {a.name}
                </span>
                <span className="network__meter" aria-hidden>
                  <span style={{ width: `${Math.round((a.rate / busiest) * 100)}%` }} />
                </span>
                <span className="network__app-rate">{a.rate > 0 ? `≈ ${formatSpeed(a.rate)}` : "idle"}</span>
                <span className="network__app-conns">
                  {a.connections} {a.connections === 1 ? "link" : "links"}
                </span>
              </li>
            ))}
          </ul>
        )}
        <p className="network__note">
          Rates are estimates — Windows only shows exact per-app traffic to administrators.
        </p>
      </div>
    </section>
  );
}

function Result({
  icon: Icon,
  label,
  value,
  busy,
}: {
  icon: typeof Timer;
  label: string;
  value: string | null;
  busy: boolean;
}) {
  return (
    <span className={`network__result${busy ? " network__result--busy" : ""}`}>
      <span className="network__result-label">
        <Icon size={11} aria-hidden /> {label}
      </span>
      <span className="network__result-value">{busy ? "…" : (value ?? "—")}</span>
    </span>
  );
}
