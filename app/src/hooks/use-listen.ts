import { useEffect, useRef } from "react";
import { type AppEvent, listenTo, logError, type Unlisten } from "@/lib/bridge";

export function useListen<T>(
  subscribed: AppEvent | readonly AppEvent[],
  handler: (payload: T) => void,
) {
  const latest = useRef(handler);
  latest.current = handler;
  const names = (
    typeof subscribed === "string" ? [subscribed] : subscribed
  ).join(" ");

  useEffect(() => {
    let cancelled = false;
    const stops: Unlisten[] = [];

    async function subscribe(event: AppEvent) {
      try {
        const unlisten = await listenTo<T>(event, (payload) =>
          latest.current(payload),
        );
        if (cancelled) {
          unlisten();
          return;
        }
        stops.push(unlisten);
      } catch (error) {
        logError(`Listener for ${event}`, error);
      }
    }
    for (const event of names.split(" ") as AppEvent[]) {
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
