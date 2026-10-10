import { useEffect, useState } from "react";
import { useListen } from "@/hooks/use-listen";
import { useOverlayFeed } from "@/hooks/use-overlay-feed";
import { api, listenTo, reportError, type Unlisten } from "@/lib/bridge";
import { notify } from "@/lib/toast";
import { useAppStore } from "@/stores/app-store";
import { useMarketListingsStore } from "@/stores/market-listings-store";
import { usePresenceStore } from "@/stores/presence-store";

function createSeenGate() {
  const seen = new Set<string>();
  return (id: string): boolean => {
    if (seen.has(id)) {
      return false;
    }
    seen.add(id);
    if (seen.size > 256) {
      const oldest = seen.values().next().value;
      if (oldest !== undefined) {
        seen.delete(oldest);
      }
    }
    return true;
  };
}

export function EventBridge() {
  const setStatus = useAppStore((state) => state.setStatus);
  const setWorld = useAppStore((state) => state.setWorld);
  const applySnapshot = useMarketListingsStore((state) => state.applySnapshot);
  const clearListings = useMarketListingsStore((state) => state.clear);
  const setPresence = usePresenceStore((state) => state.setPresence);
  const [isNew] = useState(createSeenGate);
  useOverlayFeed();

  useListen("statusUpdated", setStatus);
  useListen("inventoryUpdated", setStatus);
  useEffect(() => {
    let cancelled = false;
    let stop: Unlisten | null = null;
    const load = () => api.worldstate().then(setWorld, reportError);
    listenTo("worldStateUpdated", load).then((unlisten) => {
      if (cancelled) {
        unlisten();
        return;
      }
      stop = unlisten;
      load();
    }, reportError);
    return () => {
      cancelled = true;
      stop?.();
    };
  }, [setWorld]);
  useListen("marketUpdated", applySnapshot);
  useListen("marketSessionRejected", clearListings);
  useListen("marketPresence", setPresence);
  useListen("marketAutoClosed", (closed) => {
    const item =
      closed.quantity > 1 ? `${closed.quantity} x ${closed.item}` : closed.item;
    const titles = {
      auction: "Riven auction closed",
      sell: "Sell order closed",
      buy: "Buy order closed",
    };
    const what = {
      auction: "auction",
      sell: "sell order",
      buy: "buy order",
    };
    notify.success(titles[closed.kind], {
      description: `Closed the ${what[closed.kind]} for ${item}`,
      inGame: "market_close",
    });
  });

  useListen("coreEvent", ({ id, notice }) => {
    if (!isNew(id) || !notice) {
      return;
    }
    notify[notice.tone](notice.title, {
      description: notice.body,
      inGame: notice.in_game,
    });
  });

  return null;
}
