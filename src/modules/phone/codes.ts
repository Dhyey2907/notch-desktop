// Sign-in codes in phone notifications ("Your OTP is 482913", "G-123456 is your Google
// verification code"): found so Overline can copy them for you. Pure, so it's tested.

const HINT = /\b(code|otp|one[- ]time|verification|verify|passcode|pin|2fa|security|login|sign[- ]in)\b/i;
/** 4–8 digits on their own (a "G-" style prefix is allowed), not part of a longer number. */
const CODE = /(?:^|[^\d])(?:[A-Z]{1,3}-)?(\d{4,8})(?!\d)/;

/** The sign-in code in a notification, or null. */
export function findCode(title: string, body: string): string | null {
  const text = `${title}\n${body}`;
  if (!HINT.test(text)) return null;
  // Skip things that look like amounts, times or dates near the code.
  for (const line of text.split(/\n/)) {
    const cleaned = line
      .replace(/\b\d{1,2}[:/.]\d{1,2}([:/.]\d{2,4})?\b/g, " ")
      .replace(/[$₹€£]\s?[\d,.]+/g, " ");
    const m = cleaned.match(CODE);
    if (m) return m[1];
  }
  return null;
}

/** "••••13": enough to recognise it on the card, not enough to read it off the screen. */
export function masked(code: string): string {
  return `${"•".repeat(Math.max(0, code.length - 2))}${code.slice(-2)}`;
}
