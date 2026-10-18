// Calls among Phone Link's notifications: an incoming call gets its own card, a missed call a
// call-back card. Phone Link words them a few ways; pure, so it's tested.

export type CallKind = "incoming" | "missed";

const INCOMING = /\b(incoming|ongoing)\s+(voice\s+|video\s+|whatsapp\s+)?call\b|\bis calling\b/i;
const MISSED = /\bmissed\s+(voice\s+|video\s+|whatsapp\s+)?call/i;

export function callKind(title: string, body: string): CallKind | null {
  const text = `${title}\n${body}`;
  if (MISSED.test(text)) return "missed";
  if (INCOMING.test(text)) return "incoming";
  return null;
}

/** Who's calling: the title, unless the title is just "Incoming call" (then the body). */
export function caller(title: string, body: string): string {
  const t = title.trim();
  if (t && !callKind(t, "")) return t;
  const b = body
    .replace(INCOMING, "")
    .replace(MISSED, "")
    .replace(/^[\s·:,-]+|[\s·:,-]+$/g, "");
  return b || "Unknown caller";
}
