// Temporary M0 foundation screen: proves the command and event round trips.
// Replaced by the notch shell in M1.
import { useEffect, useState } from "react";

import { eventBus } from "./core/eventBus";
import { ipc, isIpcError } from "./core/ipc";
import type { AppInfo } from "./core/generated/AppInfo";
import "./App.css";

type PingState = { status: "idle" } | { status: "waiting"; nonce: string } | { status: "ok"; ms: number };

function App() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [ping, setPing] = useState<PingState>({ status: "idle" });

  useEffect(() => {
    ipc
      .appInfo()
      .then(setInfo)
      .catch((e: unknown) => setError(isIpcError(e) ? e.kind : "unknown"));
  }, []);

  useEffect(() => {
    if (ping.status !== "waiting") return;
    const started = performance.now();
    return eventBus.on("DiagnosticsPong", (envelope) => {
      if (envelope.correlationId === ping.nonce) {
        setPing({ status: "ok", ms: Math.round(performance.now() - started) });
      }
    });
  }, [ping]);

  const runPing = () => {
    const nonce = crypto.randomUUID();
    setPing({ status: "waiting", nonce });
    ipc.diagnosticsPing(nonce).catch(() => setPing({ status: "idle" }));
  };

  return (
    <main className="foundation">
      <h1>Notch Desktop</h1>
      <p className="muted">Foundation check</p>

      {error && <p className="error">Backend unavailable ({error})</p>}
      {info && (
        <dl>
          <dt>Version</dt>
          <dd>{info.version}</dd>
          <dt>DB schema</dt>
          <dd>v{info.dbSchemaVersion}</dd>
          <dt>Data</dt>
          <dd>{info.dataDir}</dd>
          <dt>Logs</dt>
          <dd>{info.logDir}</dd>
        </dl>
      )}

      <button onClick={runPing} disabled={ping.status === "waiting"}>
        Ping backend event
      </button>
      <p className="muted" data-testid="ping-status">
        {ping.status === "idle" && "Not run"}
        {ping.status === "waiting" && "Waiting for event…"}
        {ping.status === "ok" && `Event received in ${ping.ms} ms`}
      </p>
    </main>
  );
}

export default App;
