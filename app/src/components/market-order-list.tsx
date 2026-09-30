import { useEffect, useRef, useState } from "react";
import { GameIcon } from "@/components/game-icon";
import { EmptyNote } from "@/components/page";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { reportError } from "@/lib/bridge";
import { cn } from "@/lib/utils";
import { capitalize } from "@/lib/world";
import type { MarketItem, Order, OrderType } from "@/types";

function perTrade(order: Order) {
  return order.perTrade && order.perTrade > 1 ? order.perTrade : 1;
}

function unitPrice(order: Order) {
  return Math.round((10 * order.platinum) / perTrade(order)) / 10;
}

const VEILED_SUBTYPES = ["unrevealed", "revealed"];

function orderDetail(order: Order) {
  if (order.rank !== undefined && order.rank !== null) {
    return `Rank ${order.rank}`;
  }
  if (order.amberStars !== undefined || order.cyanStars !== undefined) {
    return `${order.cyanStars ?? 0} cyan, ${order.amberStars ?? 0} amber`;
  }
  return order.subtype ? capitalize(order.subtype) : null;
}

function whisperName(item: MarketItem, order: Order) {
  const detail = orderDetail(order);
  if (!detail) {
    return item.name;
  }
  if (order.subtype && VEILED_SUBTYPES.includes(order.subtype)) {
    return `${detail} ${item.name}`;
  }
  return `${item.name} (${detail})`;
}

function whisper(item: MarketItem, order: Order, wanted: OrderType) {
  const bundle = perTrade(order) > 1 ? ` x${perTrade(order)}` : "";
  const player = order.user?.ingameName ?? "";
  const verb = wanted === "buy" ? "buy" : "sell";
  return `/w ${player} Hi! I want to ${verb}:${bundle} "${whisperName(item, order)}" for ${order.platinum} platinum. (warframe.market)`;
}

export function OrderList({
  item,
  orders,
  loading,
  side,
  best,
  onPickPrice,
}: {
  item: MarketItem;
  orders: Order[];
  loading: boolean;
  side: OrderType;
  best: number | null;
  onPickPrice: (platinum: number) => void;
}) {
  const [copied, setCopied] = useState<string | null>(null);
  const clearCopied = useRef(0);

  useEffect(() => () => window.clearTimeout(clearCopied.current), []);

  const handleCopy = async (order: Order) => {
    const wanted = side === "sell" ? "buy" : "sell";
    try {
      await navigator.clipboard.writeText(whisper(item, order, wanted));
    } catch (error) {
      reportError(error);
      return;
    }
    setCopied(order.id);
    window.clearTimeout(clearCopied.current);
    clearCopied.current = window.setTimeout(() => setCopied(null), 1500);
  };

  if (loading) {
    return <Skeleton className="h-32 w-full" />;
  }

  if (orders.length === 0) {
    return <EmptyNote>No trader is in game with this order.</EmptyNote>;
  }

  return (
    <ul className="flex flex-col gap-1">
      {orders.map((order) => {
        const good =
          side === "buy" && best !== null && order.platinum >= best
            ? "border-accent"
            : undefined;
        const bundle = perTrade(order);
        const detail = orderDetail(order);
        return (
          <li
            key={order.id}
            className={cn("flex rounded-md border text-sm", good)}
          >
            <button
              type="button"
              onClick={() => handleCopy(order)}
              className="hover:bg-secondary flex min-w-0 flex-1 cursor-pointer flex-col gap-1 rounded-md px-2 py-1 text-left"
            >
              {copied === order.id ? (
                <span className="text-muted-foreground">Copied!</span>
              ) : (
                <>
                  <span className="flex items-center gap-2">
                    <span className="truncate">
                      {order.user?.ingameName ?? ""}
                    </span>
                    <span className="text-muted-foreground tabular-nums">
                      x{order.quantity}
                    </span>
                  </span>
                  {(bundle > 1 || detail) && (
                    <span className="flex flex-wrap gap-1">
                      {bundle > 1 && (
                        <Badge variant="secondary">{bundle} per trade</Badge>
                      )}
                      {detail && <Badge variant="muted">{detail}</Badge>}
                    </span>
                  )}
                </>
              )}
            </button>
            <button
              type="button"
              onClick={() => onPickPrice(order.platinum)}
              title="Use this price"
              className={cn(
                "hover:bg-secondary flex shrink-0 cursor-pointer items-center gap-0.5 rounded-md px-2 py-1 font-bold tabular-nums",
                side === "sell" ? "text-primary" : "text-accent",
              )}
            >
              {order.platinum}
              <GameIcon name="platinum" size={16} alt="Platinum" />
              {bundle > 1 ? ` (${unitPrice(order)})` : ""}
            </button>
          </li>
        );
      })}
    </ul>
  );
}
