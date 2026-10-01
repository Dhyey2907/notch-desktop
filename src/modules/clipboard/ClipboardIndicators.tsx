import { useEffect, useState } from "react";

import { useSettings } from "../../settings";
import { clipTitle } from "./clips";
import { latestClip, useClipboard } from "./clipboardStore";
import { useShelf } from "./shelfStore";
import "./indicators.css";

/** Hover: the latest copy. */
export function ClipboardPeek() {
  const latest = useClipboard((s) => latestClip(s.snapshot));
  const paused = useSettings((s) => s.settings?.clipboard.paused ?? false);
  return (
    <div className="module-peek">
      <span className="module-peek__title">{latest ? clipTitle(latest) : "Clipboard"}</span>
      <span className="module-peek__meta">
        {paused
          ? "History paused"
          : latest
            ? `Copied${latest.app ? ` from ${latest.app}` : ""} · Ctrl+Alt+V`
            : "Copy something and it shows up here"}
      </span>
    </div>
  );
}

/** "1:05" for seconds. */
function clock(seconds: number) {
  const m = Math.floor(seconds / 60);
  const h = Math.floor(m / 60);
  const s = String(seconds % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m % 60).padStart(2, "0")}:${s}` : `${m}:${s}`;
}

/** Bubble: a red dot and the time while recording, an arrow while something downloads. */
export function FilesCompact() {
  const recorder = useShelf((s) => s.recorder);
  const downloading = useShelf((s) => s.downloads.length > 0);
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!recorder.recording) return;
    const id = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, [recorder.recording]);
  if (recorder.recording && recorder.startedAt != null) {
    return (
      <span className="files-compact files-compact--rec" role="img" aria-label="Recording">
        <span className="files-compact__dot" />
        {clock(Math.max(0, Math.floor((now - recorder.startedAt) / 1000)))}
      </span>
    );
  }
  if (downloading) {
    return <span className="files-compact files-compact--down" role="img" aria-label="Downloading" />;
  }
  return null;
}
