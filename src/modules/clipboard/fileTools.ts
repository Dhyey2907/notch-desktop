// Which shelf tools fit which files.

/** Images the converter can read. */
export const CONVERTIBLE = ["png", "jpg", "jpeg", "webp", "gif", "bmp"];

export function isConvertible(ext: string | undefined): boolean {
  return Boolean(ext && CONVERTIBLE.includes(ext));
}

/** Converting to the format it already has isn't offered ("jpeg" counts as "jpg"). */
export function convertTargets(ext: string | undefined): ("png" | "jpg" | "webp")[] {
  const own = ext === "jpeg" ? "jpg" : ext;
  return (["png", "jpg", "webp"] as const).filter((t) => t !== own);
}

/** PDFs on the shelf, top to bottom, that can be merged. */
export function mergeablePdfs(items: { path: string; ext?: string; exists: boolean; isDir: boolean }[]) {
  return items.filter((i) => i.exists && !i.isDir && i.ext === "pdf").map((i) => i.path);
}
