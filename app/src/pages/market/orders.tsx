import type { ColumnDef } from "@tanstack/react-table";
import { Ellipsis, RefreshCw, TriangleAlert, Wrench } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { DataTable } from "@/components/data-table";
import { FilterGrid } from "@/components/filter-grid";
import { FilterSelect } from "@/components/filter-select";
import { ItemImage } from "@/components/item-image";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { itemAt } from "@/lib/arrays";
import { api } from "@/lib/bridge";
import { dateTime, num } from "@/lib/format";
import type {
  ItemSummary,
  MarketCategory,
  MarketOrders,
  OrderRow,
  OrderType,
} from "@/types";

const CATEGORIES: { value: MarketCategory; label: string }[] = [
  { value: "parts", label: "Parts" },
  { value: "relics", label: "Relics" },
  { value: "mods", label: "Mods" },
  { value: "arcanes", label: "Arcanes" },
  { value: "sets", label: "Sets" },
  { value: "misc", label: "Misc" },
];

const SIDES: { value: OrderType; label: string }[] = [
  { value: "sell", label: "Sell" },
  { value: "buy", label: "Buy" },
];

const MISSING = [
  { value: "yes", label: "Missing items" },
  { value: "no", label: "Fully stocked" },
];

interface ListedOrder {
  order: OrderRow;
  item: ItemSummary;
}

function detail(order: OrderRow) {
  if (order.rank !== null) {
    return `rank ${order.rank}`;
  }
  return order.subtype;
}

export function OrdersTable({
  orders,
  onCompare,
  onRefresh,
  onSetVisibility,
  runMarketAction,
}: {
  orders: MarketOrders;
  onCompare: (slug: string, side: OrderType, rank: number | null) => void;
  onRefresh: () => void;
  onSetVisibility: (visible: boolean) => void;
  runMarketAction: (promise: Promise<unknown>, message: string) => void;
}) {
  const { items, rows } = orders;
  const [confirming, setConfirming] = useState<"fix" | "remove" | null>(null);
  const [category, setCategory] = useState<string | null>(null);
  const [side, setSide] = useState<string | null>(null);
  const [missing, setMissing] = useState<string | null>(null);

  const entries = useMemo(
    () => rows.map((order) => ({ order, item: itemAt(items, order.item) })),
    [items, rows],
  );

  const filteredEntries = useMemo(
    () =>
      entries.filter(({ order }) => {
        if (category && order.category !== category) {
          return false;
        }
        if (side && order.order_type !== side) {
          return false;
        }
        if (missing === "yes" && !order.show_warning) {
          return false;
        }
        if (missing === "no" && order.show_warning) {
          return false;
        }
        return true;
      }),
    [entries, category, side, missing],
  );

  const columns = useMemo<ColumnDef<ListedOrder>[]>(
    () => [
      {
        id: "order_type",
        accessorFn: ({ order }) => order.order_type,
        header: "Side",
        cell: ({ row }) => {
          const side = row.original.order.order_type;
          return (
            <Badge variant={side === "sell" ? "default" : "secondary"}>
              {side}
            </Badge>
          );
        },
      },
      {
        id: "name",
        accessorFn: ({ item }) => item.name,
        header: "Item",
        enableHiding: false,
        meta: { wrap: true },
        cell: ({ row }) => {
          const { order, item } = row.original;
          return (
            <>
              <ItemImage
                imageName={item.image_name}
                size={24}
                alt={item.name}
              />
              <span className="font-medium">{item.name}</span>
              {detail(order) && <Badge variant="muted">{detail(order)}</Badge>}
            </>
          );
        },
      },
      {
        id: "category",
        accessorFn: ({ order }) => order.category,
        header: "Group",
        cell: ({ row }) => (
          <span className="text-muted-foreground">
            {row.original.order.category}
          </span>
        ),
      },
      {
        id: "platinum",
        accessorFn: ({ order }) => order.platinum,
        header: "Plat",
        meta: { numeric: true },
        cell: ({ row }) => (
          <span className="text-primary font-bold">
            {num(row.original.order.platinum)}
          </span>
        ),
      },
      {
        id: "lowest",
        accessorFn: ({ order }) => order.lowest?.plat ?? null,
        header: "Lowest",
        meta: { numeric: true, label: "Lowest price" },
        cell: ({ row }) => {
          const { lowest } = row.original.order;
          return lowest === null ? (
            <span className="text-muted-foreground">-</span>
          ) : (
            <span>
              {num(lowest.plat)}
              {lowest.from === "rank_zero" ? "+" : ""}
            </span>
          );
        },
      },
      {
        id: "quantity",
        accessorFn: ({ order }) => order.quantity,
        header: "Qty",
        meta: { numeric: true, label: "Quantity" },
        cell: ({ row }) => num(row.original.order.quantity),
      },
      {
        id: "owned",
        accessorFn: ({ order }) => order.owned,
        header: "Owned",
        meta: { numeric: true },
        cell: ({ row }) => {
          const { owned, show_warning } = row.original.order;
          return show_warning ? (
            <Badge variant="warning">
              <TriangleAlert />
              {num(owned)}
            </Badge>
          ) : (
            num(owned)
          );
        },
      },
      {
        id: "visible",
        accessorFn: ({ order }) => order.visible,
        header: "Visible",
        cell: ({ row }) =>
          row.original.order.visible ? (
            <Badge variant="accent">visible</Badge>
          ) : (
            <Badge variant="muted">hidden</Badge>
          ),
      },
      {
        id: "updated_at",
        accessorFn: ({ order }) => order.updated_at,
        header: "Updated",
        cell: ({ row }) => dateTime(row.original.order.updated_at),
      },
      {
        id: "actions",
        header: "",
        enableSorting: false,
        enableHiding: false,
        cell: ({ row }) => {
          const { order, item } = row.original;
          const slug = item.market_slug;
          return (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="size-8"
                  aria-label="Order actions"
                >
                  <Ellipsis className="size-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                {slug && (
                  <DropdownMenuItem
                    onSelect={() =>
                      onCompare(slug, order.order_type, order.rank)
                    }
                  >
                    Compare on the market
                  </DropdownMenuItem>
                )}
                {order.show_warning && (
                  <DropdownMenuItem
                    onSelect={() =>
                      runMarketAction(
                        api.marketFixOrders(order.id),
                        "Listing matched to the inventory",
                      )
                    }
                  >
                    Fix the missing items
                  </DropdownMenuItem>
                )}
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  onSelect={() =>
                    runMarketAction(
                      api.marketUpdateOrder(order.id, {
                        platinum: Math.max(1, order.platinum - 1),
                      }),
                      "Price lowered by 1",
                    )
                  }
                >
                  Lower price by 1
                </DropdownMenuItem>
                <DropdownMenuItem
                  onSelect={() =>
                    runMarketAction(
                      api.marketUpdateOrder(order.id, {
                        visible: !order.visible,
                      }),
                      order.visible ? "Order hidden" : "Order shown",
                    )
                  }
                >
                  {order.visible ? "Hide order" : "Show order"}
                </DropdownMenuItem>
                <DropdownMenuItem
                  onSelect={() =>
                    runMarketAction(
                      api.marketCloseOrder(order.id, 1),
                      "Order closed for one unit",
                    )
                  }
                >
                  Mark one as sold
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  variant="destructive"
                  onSelect={() =>
                    runMarketAction(
                      api.marketDeleteOrder(order.id),
                      "Order deleted",
                    )
                  }
                >
                  Delete order
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          );
        },
      },
    ],
    [onCompare, runMarketAction],
  );

  const active = [category, side, missing].filter(
    (value) => value !== null,
  ).length;

  const missingCount = rows.filter((row) => row.show_warning).length;

  const handleBulk = useCallback(() => {
    if (confirming === "fix") {
      runMarketAction(
        api.marketFixOrders(),
        "Listings matched to the inventory",
      );
    }
    if (confirming === "remove") {
      runMarketAction(api.marketRemoveAll(), "All orders deleted");
    }
    setConfirming(null);
  }, [confirming, runMarketAction]);

  return (
    <>
      <Dialog
        open={confirming !== null}
        onOpenChange={(open) => !open && setConfirming(null)}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>
              {confirming === "remove"
                ? "Delete Every Order"
                : "Fix the Missing Items"}
            </DialogTitle>
            <DialogDescription>
              {confirming === "remove"
                ? `All ${num(rows.length)} orders on your warframe.market profile are deleted, buy orders included. This cannot be undone.`
                : `${num(missingCount)} sell orders offer more than the account holds. Each one drops to the number owned, and an order for an item you no longer own is deleted.`}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setConfirming(null)}>
              Cancel
            </Button>
            <Button
              variant={confirming === "remove" ? "destructive" : "default"}
              onClick={handleBulk}
            >
              {confirming === "remove"
                ? "Delete All Orders"
                : "Fix Missing Items"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <DataTable
        tableId="marketOrders"
        columns={columns}
        data={filteredEntries}
        resetKey={JSON.stringify([category, side, missing])}
        searchPlaceholder="Filter orders"
        searchValue={({ item }) => item.name}
        initialSorting={[{ id: "updated_at", desc: true }]}
        rowKey={({ order }) => order.id}
        emptyMessage="No order matches these filters."
        toolbar={
          <>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="outline" size="sm">
                  <Wrench className="size-4" />
                  Bulk Actions
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start">
                <DropdownMenuItem
                  disabled={rows.length === 0}
                  onSelect={() => onSetVisibility(true)}
                >
                  Show all orders
                </DropdownMenuItem>
                <DropdownMenuItem
                  disabled={rows.length === 0}
                  onSelect={() => onSetVisibility(false)}
                >
                  Hide all orders
                </DropdownMenuItem>
                <DropdownMenuItem
                  disabled={missingCount === 0}
                  onSelect={() => setConfirming("fix")}
                >
                  Fix all missing items ({num(missingCount)})
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  disabled={rows.length === 0}
                  variant="destructive"
                  onSelect={() => setConfirming("remove")}
                >
                  Remove all orders
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
            <Button variant="outline" size="sm" onClick={onRefresh}>
              <RefreshCw className="size-4" />
              Refresh
            </Button>
          </>
        }
        filters={
          <FilterGrid
            activeFilters={active}
            onClear={() => {
              setCategory(null);
              setSide(null);
              setMissing(null);
            }}
          >
            <FilterSelect
              label="Group"
              value={category}
              options={CATEGORIES}
              onChange={setCategory}
            />
            <FilterSelect
              label="Side"
              value={side}
              options={SIDES}
              onChange={setSide}
            />
            <FilterSelect
              label="Stock"
              value={missing}
              options={MISSING}
              onChange={setMissing}
            />
          </FilterGrid>
        }
      />
    </>
  );
}
