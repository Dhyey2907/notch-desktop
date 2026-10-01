import { create } from "zustand";

import type { MailState } from "../../core/generated/MailState";

export const useMail = create<{ state: MailState | null }>(() => ({ state: null }));

/** The conversation in Gmail on the web. */
export function threadUrl(threadId: string): string {
  return `https://mail.google.com/mail/u/0/#inbox/${encodeURIComponent(threadId)}`;
}
