import { useCallback, useState } from "react";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import {
  DropdownMenuItem,
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";
import { Progress } from "@/components/ui/progress";
import { useListen } from "@/hooks/use-listen";
import { api, reportError } from "@/lib/bridge";
import { counted, num } from "@/lib/format";
import type {
  MarketBulkOutcome,
  MarketBulkProgress,
  MarketOrderRepairScope,
} from "@/types";
import type { RunMarketAction } from "./listing-controls";

export type OrderBulkAction = MarketOrderRepairScope | "remove";

const ORDER_BULK_ACTIONS: Record<
  OrderBulkAction,
  {
    menuLabel: string;
    needsPriceCheck: boolean;
    title: string;
    description: (count: number) => string;
    confirmLabel: string;
    destructive: boolean;
    done: string;
  }
> = {
  amount: {
    menuLabel: "Fix all missing items",
    needsPriceCheck: false,
    title: "Fix the Missing Items",
    description: (count) =>
      `${counted(count, "sell order offers", "sell orders offer")} more than you own. Confirming will adjust every order to the amount owned. If you do not own the item, the order will be deleted.`,
    confirmLabel: "Fix Missing Items",
    destructive: false,
    done: "matched to the inventory",
  },
  price: {
    menuLabel: "Set all prices to the lowest",
    needsPriceCheck: true,
    title: "Set Prices to the Lowest",
    description: (count) =>
      `${counted(count, "sell order is", "sell orders are")} further from the lowest price than your settings tolerate. Confirming will set the current lowest price for that item.`,
    confirmLabel: "Set Prices",
    destructive: false,
    done: "set to the lowest price",
  },
  amount_and_price: {
    menuLabel: "Fix pricing and amount issues",
    needsPriceCheck: true,
    title: "Fix Pricing and Amount Issues",
    description: (count) =>
      `${counted(count, "sell order has", "sell orders have")} a pricing or amount issue. Confirming will adjust offered amounts to owned amounts, and under- or overpriced items will be adjusted to the current lowest price.`,
    confirmLabel: "Fix Issues",
    destructive: false,
    done: "fixed",
  },
  remove: {
    menuLabel: "Remove all orders",
    needsPriceCheck: false,
    title: "Delete Every Order",
    description: (count) =>
      `${counted(count, "order")} will be deleted. This cannot be undone.`,
    confirmLabel: "Delete All Orders",
    destructive: true,
    done: "deleted",
  },
};

const ORDER_REPAIR_SCOPES: MarketOrderRepairScope[] = [
  "amount",
  "price",
  "amount_and_price",
];

function runOrderBulk(action: OrderBulkAction) {
  return action === "remove"
    ? api.marketRemoveAll()
    : api.marketRepairOrders(action);
}

export function OrderBulkMenu({
  counts,
  priceCheck,
  onSelect,
}: {
  counts: Record<OrderBulkAction, number>;
  priceCheck: boolean;
  onSelect: (action: OrderBulkAction) => void;
}) {
  const shown = ORDER_REPAIR_SCOPES.filter(
    (scope) => priceCheck || !ORDER_BULK_ACTIONS[scope].needsPriceCheck,
  );

  return (
    <>
      {shown.map((scope) => (
        <DropdownMenuItem
          key={scope}
          disabled={counts[scope] === 0}
          onSelect={() => onSelect(scope)}
        >
          {ORDER_BULK_ACTIONS[scope].menuLabel} ({num(counts[scope])})
        </DropdownMenuItem>
      ))}
      <DropdownMenuSeparator />
      <DropdownMenuItem
        disabled={counts.remove === 0}
        variant="destructive"
        onSelect={() => onSelect("remove")}
      >
        {ORDER_BULK_ACTIONS.remove.menuLabel}
      </DropdownMenuItem>
    </>
  );
}

function orderBulkOutcome(action: OrderBulkAction, outcome: MarketBulkOutcome) {
  const changed = `${counted(outcome.written, "order")} ${ORDER_BULK_ACTIONS[action].done}`;
  return outcome.ended === "stopped" ? `Stopped with ${changed}` : changed;
}

export function OrderBulkDialog({
  action,
  count,
  open,
  onClose,
  runMarketAction,
}: {
  action: OrderBulkAction;
  count: number;
  open: boolean;
  onClose: () => void;
  runMarketAction: RunMarketAction;
}) {
  const [progress, setProgress] = useState<MarketBulkProgress | null>(null);
  const [stopping, setStopping] = useState(false);
  const dialog = ORDER_BULK_ACTIONS[action];
  const running = progress !== null;
  const shownCount = progress === null ? count : progress.total;

  useListen("marketBulkProgress", (next) =>
    setProgress((current) => (current === null ? null : next)),
  );

  const handleConfirm = useCallback(async () => {
    setProgress({ written: 0, total: count });
    await runMarketAction(
      runOrderBulk(action),
      (outcome) => orderBulkOutcome(action, outcome),
      "published",
    );

    onClose();
    setProgress(null);
    setStopping(false);
  }, [action, count, onClose, runMarketAction]);

  const handleStop = useCallback(async () => {
    setStopping(true);
    try {
      await api.marketStopBulkRun();
    } catch (error) {
      reportError(error);
      setStopping(false);
    }
  }, []);

  return (
    <ConfirmDialog
      open={open}
      onOpenChange={(next) => {
        if (!next && !running) {
          onClose();
        }
      }}
      title={dialog.title}
      description={dialog.description(shownCount)}
      confirmLabel={dialog.confirmLabel}
      destructive={dialog.destructive}
      busy={stopping}
      onConfirm={handleConfirm}
      onStop={running ? handleStop : undefined}
    >
      {progress && (
        <div className="flex flex-col gap-2">
          <Progress value={(progress.written / progress.total) * 100} />
          <span className="text-muted-foreground text-sm">
            {num(progress.written)} of {num(progress.total)}
          </span>
        </div>
      )}
    </ConfirmDialog>
  );
}
