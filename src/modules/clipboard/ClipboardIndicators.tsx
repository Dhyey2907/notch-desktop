import { useSettings } from "../../settings";
import { clipTitle } from "./clips";
import { latestClip, useClipboard } from "./clipboardStore";

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
