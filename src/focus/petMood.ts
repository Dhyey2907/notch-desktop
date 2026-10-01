export type PetMood = "happy" | "working" | "sleepy" | "cheering";

/** Away this long and the pet dozes off. */
export const SLEEPY_AFTER_MS = 10 * 60_000;

export function petMood({
  cheeringUntil,
  focusing,
  idleMs,
  now,
}: {
  cheeringUntil: number;
  focusing: boolean;
  idleMs: number;
  now: number;
}): PetMood {
  if (now < cheeringUntil) return "cheering";
  if (focusing) return "working";
  if (idleMs >= SLEEPY_AFTER_MS) return "sleepy";
  return "happy";
}
