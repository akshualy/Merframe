import { useEffect, useRef } from "react";
import { type AppEvent, listenTo, logError, type Unlisten } from "@/lib/bridge";

export function useListen<T>(event: AppEvent, handler: (payload: T) => void) {
  const latest = useRef(handler);
  latest.current = handler;

  useEffect(() => {
    let cancelled = false;
    let stop: Unlisten | null = null;

    async function subscribe() {
      try {
        const unlisten = await listenTo<T>(event, (payload) =>
          latest.current(payload),
        );
        if (cancelled) {
          unlisten();
          return;
        }
        stop = unlisten;
      } catch (error) {
        logError(`Listener for ${event}`, error);
      }
    }
    subscribe();

    return () => {
      cancelled = true;
      stop?.();
      stop = null;
    };
  }, [event]);
}
