// Frontend side of the typed event bus (architecture.md §6). One Tauri listener on
// the `app-event` channel fans envelopes out to subscribers keyed by event type.
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { AppEvent } from "./generated/AppEvent";
import type { EventEnvelope } from "./generated/EventEnvelope";

export const APP_EVENT_CHANNEL = "app-event";

export type AppEventType = AppEvent["type"];

/** Envelope narrowed to a single event type, so handlers get a typed payload. */
export type EnvelopeOf<T extends AppEventType> = Omit<EventEnvelope, "event"> & {
  event: Extract<AppEvent, { type: T }>;
};

type Handler = (envelope: EventEnvelope) => void;

export class EventBus {
  private handlers = new Map<AppEventType | "*", Set<Handler>>();

  /** Subscribe to one event type. Returns an unsubscribe function. */
  on<T extends AppEventType>(type: T, handler: (envelope: EnvelopeOf<T>) => void): () => void {
    return this.add(type, handler as Handler);
  }

  /** Subscribe to every event (diagnostics, activity engine). */
  onAny(handler: Handler): () => void {
    return this.add("*", handler);
  }

  dispatch(envelope: EventEnvelope): void {
    for (const key of [envelope.event.type, "*"] as const) {
      this.handlers.get(key)?.forEach((handler) => {
        try {
          handler(envelope);
        } catch (error) {
          // A failing subscriber must not stop delivery to the others.
          console.error(`event handler for ${envelope.event.type} failed`, error);
        }
      });
    }
  }

  private add(key: AppEventType | "*", handler: Handler): () => void {
    let set = this.handlers.get(key);
    if (!set) {
      set = new Set();
      this.handlers.set(key, set);
    }
    set.add(handler);
    return () => set.delete(handler);
  }
}

export const eventBus = new EventBus();

/** Connect the shared bus to the backend channel. Call once at startup. */
export function connectEventBus(bus: EventBus = eventBus): Promise<UnlistenFn> {
  return listen<EventEnvelope>(APP_EVENT_CHANNEL, (event) => bus.dispatch(event.payload));
}
