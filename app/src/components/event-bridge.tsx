import { useState } from "react";
import { useListen } from "@/hooks/use-listen";
import { useOverlayFeed } from "@/hooks/use-overlay-feed";
import { api, events, reportError } from "@/lib/bridge";
import { countdown, num } from "@/lib/format";
import { notify } from "@/lib/toast";
import { useAppStore } from "@/stores/app-store";
import { useMarketListingsStore } from "@/stores/market-listings-store";
import { usePresenceStore } from "@/stores/presence-store";
import type {
  CoreEvent,
  CoreEventEnvelope,
  GameStatus,
  MarketAutoClose,
  MarketSnapshot,
  Presence,
} from "@/types";

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

  useListen<GameStatus>(events.statusUpdated, setStatus);
  useListen<GameStatus>(events.inventoryUpdated, setStatus);
  useListen(events.worldStateUpdated, async () => {
    try {
      setWorld(await api.worldstate());
    } catch (error) {
      reportError(error);
    }
  });
  useListen<MarketSnapshot>(events.marketUpdated, applySnapshot);
  useListen(events.marketSignedOut, clearListings);
  useListen<Presence>(events.marketPresence, setPresence);
  useListen<MarketAutoClose>(events.marketAutoClosed, (closed) => {
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

  const show = (event: CoreEvent) => {
    if ("InventoryUpdated" in event) {
      const summary = event.InventoryUpdated;
      if (summary.changes > 0) {
        notify.info(`${summary.changes} inventory changes`, {
          description: `${num(summary.plat)} plat, ${num(summary.endo)} endo, MR ${summary.mr}`,
          inGame: "inventory",
        });
      }
      return;
    }
    if ("TradeCompleted" in event) {
      const { trade, partner } = event.TradeCompleted;
      notify.success("Trade completed", {
        description: `${trade.plat} plat with ${partner ?? "an unknown Tenno"}`,
        inGame: "trade",
      });
      return;
    }
    if ("NewConversation" in event) {
      notify.info("New in-game conversation", {
        description: event.NewConversation.player,
      });
      return;
    }
    if ("RelicRewardScreen" in event) {
      return;
    }
    if ("FissureAlert" in event) {
      const { fissure } = event.FissureAlert;
      const levels = fissure.levels
        ? ` (${fissure.levels[0]}-${fissure.levels[1]})`
        : "";
      const faction = fissure.faction ? ` - ${fissure.faction}` : "";
      const steelPath = fissure.steel_path ? ", Steel Path" : "";
      const kind = fissure.is_storm ? "Void Storm" : "Fissure";
      notify.warning(
        `New ${fissure.tier} ${kind} - ${fissure.node_name ?? fissure.node_id}`,
        {
          description: `${fissure.mission_name}${levels}${faction}${steelPath}, ${countdown(fissure.remaining_secs)} left`,
          inGame: "fissure",
        },
      );
      return;
    }
    const timer = event.TimerAlert;
    notify.warning(`${timer.name} turns ${timer.next_state}`, {
      description: `in ${countdown(timer.remaining_secs)}`,
      inGame: "timer",
    });
  };

  useListen<CoreEventEnvelope>(events.coreEvent, (envelope) => {
    if (!isNew(envelope.id)) {
      return;
    }
    show(envelope.event);
  });

  return null;
}
