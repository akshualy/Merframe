import type { ColumnDef } from "@tanstack/react-table";
import { Ellipsis, TriangleAlert, Wrench } from "lucide-react";
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
import { api } from "@/lib/bridge";
import { dateTime, num } from "@/lib/format";
import type { MarketCategory, OrderRow, OrderType } from "@/types";

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

function detail(row: OrderRow) {
  if (row.rank !== null) {
    return `rank ${row.rank}`;
  }
  return row.subtype;
}

export function OrdersTable({
  rows,
  onCompare,
  runMarketAction,
}: {
  rows: OrderRow[];
  onCompare: (slug: string) => void;
  runMarketAction: (promise: Promise<unknown>, message: string) => void;
}) {
  const [confirming, setConfirming] = useState<"fix" | "remove" | null>(null);
  const [category, setCategory] = useState<string | null>(null);
  const [side, setSide] = useState<string | null>(null);
  const [missing, setMissing] = useState<string | null>(null);

  const filteredRows = useMemo(
    () =>
      rows.filter((row) => {
        if (category && row.category !== category) {
          return false;
        }
        if (side && row.order_type !== side) {
          return false;
        }
        if (missing === "yes" && !row.show_warning) {
          return false;
        }
        if (missing === "no" && row.show_warning) {
          return false;
        }
        return true;
      }),
    [rows, category, side, missing],
  );

  const columns = useMemo<ColumnDef<OrderRow>[]>(
    () => [
      {
        accessorKey: "order_type",
        header: "Side",
        cell: ({ row }) => (
          <Badge
            variant={
              row.original.order_type === "sell" ? "default" : "secondary"
            }
          >
            {row.original.order_type}
          </Badge>
        ),
      },
      {
        accessorKey: "name",
        header: "Item",
        enableHiding: false,
        meta: { wrap: true },
        cell: ({ row }) => (
          <>
            <ItemImage
              imageName={row.original.image_name}
              size={24}
              alt={row.original.name}
            />
            <span className="font-medium">{row.original.name}</span>
            {detail(row.original) && (
              <Badge variant="muted">{detail(row.original)}</Badge>
            )}
          </>
        ),
      },
      {
        accessorKey: "category",
        header: "Group",
        cell: ({ row }) => (
          <span className="text-muted-foreground">{row.original.category}</span>
        ),
      },
      {
        accessorKey: "platinum",
        header: "Plat",
        meta: { numeric: true },
        cell: ({ row }) => (
          <span className="text-primary font-bold">
            {num(row.original.platinum)}
          </span>
        ),
      },
      {
        accessorKey: "lowest",
        header: "Lowest",
        meta: { numeric: true, label: "Lowest price" },
        cell: ({ row }) =>
          row.original.lowest === null ? (
            <span className="text-muted-foreground">-</span>
          ) : (
            <span>
              {num(row.original.lowest)}
              {row.original.lowest_from_rank_zero ? "+" : ""}
            </span>
          ),
      },
      {
        accessorKey: "quantity",
        header: "Qty",
        meta: { numeric: true, label: "Quantity" },
        cell: ({ row }) => num(row.original.quantity),
      },
      {
        accessorKey: "owned",
        header: "Owned",
        meta: { numeric: true },
        cell: ({ row }) =>
          row.original.show_warning ? (
            <Badge variant="warning">
              <TriangleAlert />
              {num(row.original.owned)}
            </Badge>
          ) : (
            num(row.original.owned)
          ),
      },
      {
        accessorKey: "visible",
        header: "Visible",
        cell: ({ row }) =>
          row.original.visible ? (
            <Badge variant="accent">visible</Badge>
          ) : (
            <Badge variant="muted">hidden</Badge>
          ),
      },
      {
        accessorKey: "updated_at",
        header: "Updated",
        cell: ({ row }) => dateTime(row.original.updated_at),
      },
      {
        id: "actions",
        header: "",
        enableSorting: false,
        enableHiding: false,
        cell: ({ row }) => (
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
              <DropdownMenuItem onSelect={() => onCompare(row.original.slug)}>
                Compare on the market
              </DropdownMenuItem>
              {row.original.show_warning && (
                <DropdownMenuItem
                  onSelect={() =>
                    runMarketAction(
                      api.marketFixOrders(row.original.id),
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
                    api.marketUpdateOrder(row.original.id, {
                      platinum: Math.max(1, row.original.platinum - 1),
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
                    api.marketUpdateOrder(row.original.id, {
                      visible: !row.original.visible,
                    }),
                    row.original.visible ? "Order hidden" : "Order shown",
                  )
                }
              >
                {row.original.visible ? "Hide order" : "Show order"}
              </DropdownMenuItem>
              <DropdownMenuItem
                onSelect={() =>
                  runMarketAction(
                    api.marketCloseOrder(row.original.id, 1),
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
                    api.marketDeleteOrder(row.original.id),
                    "Order deleted",
                  )
                }
              >
                Delete order
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        ),
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
        data={filteredRows}
        searchPlaceholder="Filter orders"
        searchValue={(row) => row.name}
        initialSorting={[{ id: "updated_at", desc: true }]}
        rowKey={(row) => row.id}
        pageSize={25}
        emptyMessage="No order matches these filters."
        toolbar={
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" size="sm">
                <Wrench className="size-4" />
                Bulk Actions
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start">
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
