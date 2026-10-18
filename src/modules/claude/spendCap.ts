// The Claude daily spend cap (Settings → Developer): which warning step today's estimated spend
// has reached. Pure, so it's tested; the module remembers the step it last warned about per day.

export type CapStep = 0 | 80 | 100;

export function capStep(spentUsd: number, capUsd: number): CapStep {
  if (!(capUsd > 0)) return 0;
  if (spentUsd >= capUsd) return 100;
  if (spentUsd >= capUsd * 0.8) return 80;
  return 0;
}

/** A step worth a card: higher than the one already shown today. */
export function newStep(step: CapStep, shown: CapStep): boolean {
  return step > shown;
}

const KEY = "overline.claudeCap";

/** The step already warned about today (localStorage; 0 when unknown or another day). */
export function shownStep(today: string): CapStep {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "null") as { day?: string; step?: CapStep } | null;
    return raw?.day === today ? (raw.step ?? 0) : 0;
  } catch {
    return 0;
  }
}

export function rememberStep(today: string, step: CapStep) {
  try {
    localStorage.setItem(KEY, JSON.stringify({ day: today, step }));
  } catch {
    // Private mode or storage off: at worst the card shows again.
  }
}
