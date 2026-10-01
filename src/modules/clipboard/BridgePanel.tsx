// The phone bridge's QR code: scan it with the phone's camera to download the offered files
// or send files back. It ends on Stop or after 10 minutes.
import { Smartphone, Square } from "lucide-react";
import { useEffect, useState } from "react";

import type { BridgeState } from "../../core/generated/BridgeState";
import { ipc } from "../../core/ipc";
import { Button } from "../../design/components";

function useSecondNow(): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, []);
  return now;
}

export function BridgePanel({ bridge }: { bridge: BridgeState }) {
  const now = useSecondNow();
  const left = Math.max(0, Date.parse(bridge.expiresAt) - now);
  const minutes = Math.floor(left / 60_000);
  const seconds = String(Math.floor((left % 60_000) / 1000)).padStart(2, "0");
  // The SVG comes from our own QR encoder; shown as an image, never inserted as HTML.
  const qr = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(bridge.qrSvg)}`;
  return (
    <div className="bridge">
      <img className="bridge__qr" src={qr} alt="QR code for your phone" />
      <div className="bridge__text">
        <p className="bridge__title">
          <Smartphone size={14} aria-hidden /> Scan with your phone
        </p>
        <p className="bridge__meta">
          {bridge.files.length
            ? `${bridge.files.length} ${bridge.files.length === 1 ? "file" : "files"} to download · or send files back`
            : "Send files from your phone to this PC"}
        </p>
        <p className="bridge__meta">
          Same Wi-Fi only · ends in {minutes}:{seconds}
          {bridge.received > 0 && ` · ${bridge.received} received`}
        </p>
        <code className="bridge__url">{bridge.url.replace(/\/[0-9a-f]{32}\/$/, "/…")}</code>
        <Button onClick={() => void ipc.bridgeStop()}>
          <Square size={12} aria-hidden /> Stop
        </Button>
      </div>
    </div>
  );
}
