import { useState } from "react";
import { toast } from "sonner";
import { useListen } from "@/hooks/use-listen";
import { useOverlayFeed } from "@/hooks/use-overlay-feed";
import { api, events, reportError } from "@/lib/bridge";
import { countdown, num } from "@/lib/format";
import { useAppStore } from "@/stores/app-store";
import type {
  CoreEvent,
  CoreEventEnvelope,
  GameStatus,
  MarketAutoClose,
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
  const { setStatus, setWorld } = useAppStore();
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
    toast.success(titles[closed.kind], {
      description: `Closed the ${what[closed.kind]} for ${item}`,
    });
  });

  const show = (event: CoreEvent) => {
    if ("InventoryUpdated" in event) {
      const summary = event.InventoryUpdated;
      if (summary.changes > 0) {
        toast.info(`${summary.changes} inventory changes`, {
          description: `${num(summary.plat)} plat, ${num(summary.endo)} endo, MR ${summary.mr}`,
        });
      }
      return;
    }
    if ("TradeCompleted" in event) {
      const { trade, partner } = event.TradeCompleted;
      toast.success("Trade completed", {
        description: `${trade.plat} plat with ${partner ?? "an unknown Tenno"}`,
      });
      return;
    }
    if ("NewConversation" in event) {
      toast.info("New in-game conversation", {
        description: event.NewConversation.player,
      });
      return;
    }
    if ("RelicRewardScreen" in event) {
      return;
    }
    if ("FissureAlert" in event) {
      const { fissure } = event.FissureAlert;
      toast.warning(`${fissure.tier} ${fissure.mission_name} fissure`, {
        description: `${fissure.planet ?? fissure.node_name ?? fissure.node_id}${fissure.steel_path ? " (Steel Path)" : ""}, ${countdown(fissure.remaining_secs)} left`,
      });
      return;
    }
    const timer = event.TimerAlert;
    toast.warning(`${timer.name} turns ${timer.next_state}`, {
      description: `in ${countdown(timer.remaining_secs)}`,
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
