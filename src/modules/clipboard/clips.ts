import type { Clip } from "../../core/generated/Clip";

/** Case-insensitive search over what the list shows: text, file names, app. */
export function matchesQuery(clip: Clip, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  const haystack = [clip.preview, clip.app ?? "", clip.kind === "image" ? "image picture screenshot" : ""]
    .join("\n")
    .toLowerCase();
  return q.split(/\s+/).every((word) => haystack.includes(word));
}

/** One line for the list: text squashed onto a line, or a description of the clip. */
export function clipTitle(clip: Clip): string {
  switch (clip.kind) {
    case "image":
      return clip.width && clip.height ? `Image · ${clip.width}×${clip.height}` : "Image";
    case "files":
      return clip.files.length === 1 ? clip.preview : `${clip.files.length} files · ${clip.preview}`;
    default:
      return clip.preview.replace(/\s+/g, " ").trim();
  }
}

/** Plain text worth an "Add as a task" (short, one line; not a link, color or JSON). */
export function taskTitle(clip: Clip): string | null {
  if (clip.kind !== "text" || clip.textKind) return null;
  const line = clip.preview.trim();
  return line && !line.includes("\n") && line.length <= 200 ? line : null;
}

/** "now", "5m", "2h", "3d". */
export function age(at: string, now: number): string {
  const minutes = Math.floor((now - new Date(at).getTime()) / 60_000);
  if (minutes < 1) return "now";
  if (minutes < 60) return `${minutes}m`;
  if (minutes < 24 * 60) return `${Math.floor(minutes / 60)}h`;
  return `${Math.floor(minutes / (24 * 60))}d`;
}
