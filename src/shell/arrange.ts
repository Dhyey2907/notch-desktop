// Pure helpers for drag to rearrange (Arrange.tsx).

/** The unit nearest the pointer (by distance to each unit's centre). */
export function nearestUnit(point: { x: number; y: number }, centers: readonly { x: number; y: number }[]) {
  let best = -1;
  let bestDistance = Infinity;
  centers.forEach((c, i) => {
    const d = Math.hypot(point.x - c.x, point.y - c.y);
    if (d < bestDistance) {
      best = i;
      bestDistance = d;
    }
  });
  return best;
}

/**
 * The full order to save: the units' ids in their new order, then everything else the group
 * knows (hidden sections) in its old order, so hiding and showing keep working.
 */
export function savedOrder(units: readonly { ids: readonly string[] }[], all: readonly string[]): string[] {
  const shown = units.flatMap((u) => u.ids);
  return [...shown, ...all.filter((id) => !shown.includes(id))];
}
