// Claude → Sessions → New task: pick a project Claude Code has worked in, type the task, and a
// terminal opens there with Claude already on it.
import { Play } from "lucide-react";
import { useEffect, useState } from "react";

import { ipc, isIpcError } from "../../core/ipc";
import { Button } from "../../design/components";

/** "C:\dev\notch-desktop" → "notch-desktop". */
function folderName(path: string): string {
  return (
    path
      .replace(/[\\/]+$/, "")
      .split(/[\\/]/)
      .pop() || path
  );
}

export function NewTask() {
  const [projects, setProjects] = useState<string[]>([]);
  const [cwd, setCwd] = useState("");
  const [task, setTask] = useState("");
  const [note, setNote] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    ipc.claudeProjects().then(
      (list) => {
        if (!live) return;
        setProjects(list);
        setCwd((c) => c || list[0] || "");
      },
      () => {},
    );
    return () => {
      live = false;
    };
  }, []);

  if (projects.length === 0) return null;

  const start = () =>
    void ipc.claudeLaunch(cwd, task).then(
      () => {
        setNote(`Started in ${folderName(cwd)}`);
        setTask("");
      },
      (e) => setNote(isIpcError(e) ? e.message : "Couldn't start it"),
    );

  return (
    <form
      className="newtask"
      onSubmit={(e) => {
        e.preventDefault();
        start();
      }}
    >
      <select
        className="newtask__project"
        aria-label="Project"
        value={cwd}
        title={cwd}
        onChange={(e) => setCwd(e.target.value)}
      >
        {projects.map((p) => (
          <option key={p} value={p}>
            {folderName(p)}
          </option>
        ))}
      </select>
      <input
        className="newtask__input"
        aria-label="Task for Claude Code"
        placeholder="New task for Claude Code (or leave empty to just open it)"
        value={task}
        onChange={(e) => setTask(e.target.value)}
        onKeyDown={(e) => e.stopPropagation()}
      />
      <Button type="submit" variant="primary">
        <Play size={12} aria-hidden /> Start
      </Button>
      {note && (
        <p className="newtask__note" role="status">
          {note}
        </p>
      )}
    </form>
  );
}
