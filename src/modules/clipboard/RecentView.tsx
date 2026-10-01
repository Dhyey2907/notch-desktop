// Files opened recently in any app (Windows' Recent list): drag one out, open it, show it in
// its folder, or keep it on the shelf.
import { FolderOpen, PackagePlus } from "lucide-react";
import { useEffect, useState } from "react";

import type { RecentFile } from "../../core/generated/RecentFile";
import { ipc, isIpcError } from "../../core/ipc";
import { EmptyState, IconButton } from "../../design/components";
import { age } from "./clips";
import { FileIcon } from "./FileIcon";
import { formatSize } from "./shelfStore";
import { useDragOut } from "./useDragOut";
import { useMinuteClock } from "./useMinuteClock";

function message(error: unknown) {
  return isIpcError(error)
    ? error.message.replace(/^invalid input: /, "")
    : "Something went wrong. Try again.";
}

export function RecentView() {
  const [files, setFiles] = useState<RecentFile[] | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const now = useMinuteClock();

  useEffect(() => {
    let alive = true;
    ipc.filesRecent().then(
      (list) => alive && setFiles(list),
      () => alive && setFiles([]),
    );
    return () => {
      alive = false;
    };
  }, []);

  if (files === null) return <div className="clips" />;
  if (files.length === 0) {
    return (
      <EmptyState
        title="Nothing opened recently"
        body="Files you open in any app show up here, ready to drag into another one."
      />
    );
  }
  return (
    <div className="clips">
      {status && <p className="clips-status">{status}</p>}
      <ul className="clips-list" aria-label="Recently opened files">
        {files.map((file) => (
          <RecentRow key={file.path} file={file} now={now} onStatus={setStatus} />
        ))}
      </ul>
    </div>
  );
}

function RecentRow({
  file,
  now,
  onStatus,
}: {
  file: RecentFile;
  now: number;
  onStatus: (text: string | null) => void;
}) {
  const drag = useDragOut(
    () => ipc.filesDrag([file.path]),
    (e) => onStatus(message(e)),
  );
  const run = (action: Promise<unknown>, done?: string) =>
    void action.then(
      () => onStatus(done ?? null),
      (e) => onStatus(message(e)),
    );
  const meta = [
    file.ext?.toUpperCase(),
    formatSize(file.size),
    age(new Date(file.openedAt).toISOString(), now),
  ]
    .filter(Boolean)
    .join(" · ");
  return (
    <li className="clip shelf-item" title={file.path}>
      <div
        className="clip__main shelf-item__drag"
        {...drag}
        onDoubleClick={() => run(ipc.filesOpen(file.path))}
      >
        <span className="clip__icon" aria-hidden>
          <FileIcon ext={file.ext} />
        </span>
        <span className="clip__text">
          <span className="clip__title">{file.name}</span>
          <span className="clip__meta">{meta}</span>
        </span>
      </div>
      <div className="clip__actions">
        <IconButton
          icon={PackagePlus}
          label={`Keep ${file.name} on the shelf`}
          size="sm"
          onClick={() => run(ipc.shelfAdd([file.path]), "On the Files shelf")}
        />
        <IconButton
          icon={FolderOpen}
          label="Show in folder"
          size="sm"
          onClick={() => run(ipc.filesReveal(file.path))}
        />
      </div>
    </li>
  );
}
