import { useEffect, useRef } from "react";
import {
  type EventData,
  type EventName,
  listenTo,
  logError,
  type Unlisten,
} from "@/lib/bridge";

export function useListen<K extends EventName>(
  subscribed: K | readonly K[],
  handler: (data: EventData<K>) => void,
) {
  const latest = useRef(handler);
  latest.current = handler;
  const names = (
    typeof subscribed === "string" ? [subscribed] : subscribed
  ).join(" ");

  useEffect(() => {
    let cancelled = false;
    const stops: Unlisten[] = [];

    async function subscribe(event: K) {
      try {
        const unlisten = await listenTo(event, (data) => latest.current(data));
        if (cancelled) {
          unlisten();
          return;
        }
        stops.push(unlisten);
      } catch (error) {
        logError(`Listener for ${event}`, error);
      }
    }
    for (const event of names.split(" ") as K[]) {
      subscribe(event);
    }

    return () => {
      cancelled = true;
      for (const stop of stops.splice(0)) {
        stop();
      }
    };
  }, [names]);
}
