// The bubble while a meeting is recorded (a red dot and the time) or its notes are being made.
import { useEffect, useState } from "react";

import { elapsed, useMeeting } from "./meetingStore";

export function MeetingCompact() {
  const state = useMeeting((s) => s.state);
  const recording = state.phase === "recording" && state.startedAt != null;
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!recording) return;
    const id = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, [recording]);
  if (recording && state.startedAt != null)
    return (
      <span className="files-compact files-compact--rec" role="img" aria-label="Recording a meeting">
        <span className="files-compact__dot" />
        {elapsed(state.startedAt, now)}
      </span>
    );
  if (state.phase === "transcribing" || state.phase === "summarizing")
    return (
      <span className="files-compact" role="img" aria-label="Making meeting notes">
        ✎
      </span>
    );
  return null;
}
