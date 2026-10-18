// Arrow-key movement in the window switcher's grid. Pure, so it's tested.

export const COLUMNS = 4;

/** Index after moving by an arrow key in a grid of `count` tiles. */
export function moveIndex(index: number, key: string, count: number, columns = COLUMNS): number {
  if (count === 0) return 0;
  const step =
    ({ ArrowLeft: -1, ArrowRight: 1, ArrowUp: -columns, ArrowDown: columns } as Record<string, number>)[
      key
    ] ?? 0;
  const next = index + step;
  if (key === "ArrowLeft" || key === "ArrowRight") return (next + count) % count;
  return next < 0 || next >= count ? index : next;
}
