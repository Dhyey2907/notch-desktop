// Meeting notes: set up once (download Whisper and its speech model), then Start / Stop. The
// notes come back as a card; they're saved to Notes and the action items to To-do.
import { Download, Mic, Square, X } from "lucide-react";
import { useEffect, useState } from "react";

import { ipc, isIpcError } from "../../core/ipc";
import { Button, EmptyState, ModuleContainer, Progress } from "../../design/components";
import { elapsed, useMeeting } from "./meetingStore";
import "./meeting.css";

const PHASE_TEXT: Record<string, string> = {
  transcribing: "Turning the recording into text, here on this PC…",
  summarizing: "Writing the summary and action items…",
};

export default function MeetingExpanded() {
  const state = useMeeting((s) => s.state);
  const [error, setError] = useState<string | null>(null);
  const [asked, setAsked] = useState(false);
  const [now, setNow] = useState(() => Date.now());
  const recording = state.phase === "recording" && state.startedAt != null;

  useEffect(() => {
    if (!recording) return;
    const id = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, [recording]);

  const run = (p: Promise<unknown>) =>
    void p.then(
      () => setError(null),
      (e) => setError(isIpcError(e) ? e.message.replace(/^invalid input: /, "") : "That didn't work"),
    );

  let body;
  if (state.phase === "installing") {
    body = (
      <div className="meeting">
        <p>Downloading Whisper and its speech model…</p>
        <Progress value={state.progress ?? 0} max={100} label="Download progress" />
      </div>
    );
  } else if (!state.ready) {
    body = asked ? (
      <div className="meeting">
        <p className="meeting__title">Download the speech model?</p>
        <p className="meeting__sub">
          About 150 MB, once: whisper.cpp from its GitHub releases and the “base” model from its Hugging Face
          page. Meetings are then turned into text on this PC; no audio is uploaded.
        </p>
        <div className="meeting__row">
          <Button onClick={() => setAsked(false)}>Not now</Button>
          <Button variant="primary" onClick={() => run(ipc.meetingInstall())}>
            <Download size={13} aria-hidden /> Download
          </Button>
        </div>
      </div>
    ) : (
      <EmptyState
        title="Meeting notes"
        body="Record a meeting (your mic and the call's sound), get it written out here on your PC, then a summary and action items."
        action={
          <Button variant="primary" onClick={() => setAsked(true)}>
            Set up
          </Button>
        }
      />
    );
  } else if (recording && state.startedAt != null) {
    body = (
      <div className="meeting">
        <p className="meeting__title">
          <span className="meeting__dot" aria-hidden /> Recording · {elapsed(state.startedAt, now)}
        </p>
        <p className="meeting__sub">
          Your microphone and what the PC plays. Tell the others you're taking notes.
        </p>
        <div className="meeting__row">
          <Button onClick={() => run(Promise.resolve(ipc.meetingCancel()))}>
            <X size={13} aria-hidden /> Discard
          </Button>
          <Button variant="primary" onClick={() => run(ipc.meetingStop())}>
            <Square size={13} aria-hidden /> Stop and make notes
          </Button>
        </div>
      </div>
    );
  } else if (PHASE_TEXT[state.phase]) {
    body = (
      <div className="meeting">
        <p>{PHASE_TEXT[state.phase]}</p>
        <p className="meeting__sub">A card will say when the notes are ready.</p>
      </div>
    );
  } else {
    body = (
      <div className="meeting">
        <p className="meeting__sub">
          Records your microphone and the PC's sound. When you stop, it's written out on this PC, then
          summarised with action items (saved to Notes and To-do).
        </p>
        {state.error && <p className="meeting__error">Last time: {state.error}</p>}
        <div className="meeting__row">
          <Button variant="primary" onClick={() => run(ipc.meetingStart())}>
            <Mic size={13} aria-hidden /> Start taking notes
          </Button>
        </div>
      </div>
    );
  }

  return (
    <ModuleContainer title="Meeting notes">
      {body}
      {error && <p className="meeting__error">{error}</p>}
    </ModuleContainer>
  );
}
