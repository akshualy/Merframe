import { useEffect } from "react";
import { useListen } from "@/hooks/use-listen";
import { api, events, logError } from "@/lib/bridge";
import { useOverlayStore } from "@/stores/overlay-store";
import type { OverlayState } from "@/types";

export function useOverlayFeed(): OverlayState {
  const { overlays, setOverlays, setLoaded } = useOverlayStore();
  useListen<OverlayState>(events.overlayState, setOverlays);

  useEffect(() => {
    const load = async () => {
      try {
        const current = await api.overlayState();
        if (current.seq >= useOverlayStore.getState().overlays.seq) {
          setOverlays(current);
        }
      } catch (error) {
        logError("Loading the overlay state failed", error);
      } finally {
        setLoaded();
      }
    };
    load();
  }, [setOverlays, setLoaded]);

  return overlays;
}
