// Fair picks for the decision maker, from the system's secure random source.

/** A whole number in [0, n), without the bias of `random % n`. */
export function randomBelow(
  n: number,
  source: (out: Uint32Array<ArrayBuffer>) => void = (out) => crypto.getRandomValues(out),
): number {
  if (!Number.isInteger(n) || n <= 0) return 0;
  const limit = Math.floor(0x1_0000_0000 / n) * n;
  const buf = new Uint32Array(1);
  for (;;) {
    source(buf);
    if (buf[0] < limit) return buf[0] % n;
  }
}

/** "pizza, sushi,  tacos" → ["pizza", "sushi", "tacos"] (no blanks, no repeats, at most 12). */
export function parseOptions(text: string): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const part of text.split(/[,\n]/)) {
    const option = part.trim().slice(0, 40);
    const key = option.toLowerCase();
    if (!option || seen.has(key)) continue;
    seen.add(key);
    out.push(option);
    if (out.length === 12) break;
  }
  return out;
}

/**
 * The wheel's final rotation (degrees) so slice `index` of `count` stops under the pointer at
 * the top, after a few full turns from `from`.
 */
export function wheelRotation(from: number, index: number, count: number, turns = 5): number {
  const slice = 360 / count;
  // Slice i spans [i·slice, (i+1)·slice) clockwise from the top; rotating the wheel by −centre
  // brings its centre to the top.
  const target = (360 - (index + 0.5) * slice) % 360;
  const base = from - (((from % 360) + 360) % 360);
  return base + turns * 360 + target;
}
