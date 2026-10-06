import type { ColumnDef } from "@tanstack/react-table";
import { useCallback } from "react";
import { DataTable } from "@/components/data-table";
import { GameIcon } from "@/components/game-icon";
import {
  EmptyPanel,
  ErrorNote,
  Section,
  Stat,
  TableSkeleton,
} from "@/components/page";
import { sinceMs, TimeframeTabs } from "@/components/timeframe-tabs";
import { TradesSection } from "@/components/trades-section";
import { useAsyncData } from "@/hooks/use-async-data";
import { api } from "@/lib/bridge";
import { changeTone } from "@/lib/chart";
import { num } from "@/lib/format";
import { cn } from "@/lib/utils";
import { usePreferencesStore } from "@/stores/preferences-store";
import type { CategoryStatement, TradedTotal } from "@/types";
import { CATEGORY_LABELS } from "./categories";
import { itemColumn, totalValueColumn } from "./columns";

const profitOf = (entry: CategoryStatement) => entry.revenue - entry.expenses;

const CATEGORY_COLUMNS: ColumnDef<CategoryStatement>[] = [
  {
    id: "category",
    header: "Category",
    enableHiding: false,
    accessorFn: (row) => CATEGORY_LABELS[row.category],
  },
  {
    accessorKey: "revenue",
    header: "Revenue",
    meta: { numeric: true },
    cell: ({ row }) => num(row.original.revenue),
  },
  {
    accessorKey: "expenses",
    header: "Expenses",
    meta: { numeric: true },
    cell: ({ row }) => num(row.original.expenses),
  },
  {
    id: "profit",
    header: "Profit",
    meta: { numeric: true },
    accessorFn: profitOf,
    cell: ({ getValue }) => {
      const profit = getValue<number>();
      return (
        <span className={cn("font-medium", changeTone(profit))}>
          {profit > 0 ? "+" : ""}
          {num(profit)}
        </span>
      );
    },
  },
];

const TRADED_COLUMNS: ColumnDef<TradedTotal>[] = [
  itemColumn(),
  {
    accessorKey: "amount",
    header: "Amount",
    meta: { numeric: true },
    cell: ({ row }) => num(row.original.amount),
  },
  totalValueColumn(),
];

function Platinum({ value, tone }: { value: number; tone?: string }) {
  return (
    <span className={cn("flex items-center gap-1.5", tone)}>
      {num(value)}
      <GameIcon name="platinum" size={16} alt="Platinum" />
    </span>
  );
}

export function TradeHistory() {
  const { timeframe } = usePreferencesStore();
  const load = useCallback(
    () => api.tradeAnalytics(sinceMs(timeframe)),
    [timeframe],
  );
  const { data, error } = useAsyncData(load, ["inventoryUpdated"]);

  if (!data) {
    return error ? <ErrorNote message={error} /> : <TableSkeleton />;
  }

  const revenue = data.categories.reduce(
    (sum, entry) => sum + entry.revenue,
    0,
  );
  const expenses = data.categories.reduce(
    (sum, entry) => sum + entry.expenses,
    0,
  );

  return (
    <>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2 className="text-foreground text-lg font-bold">Your Trades</h2>
        <TimeframeTabs />
      </div>

      {error && <ErrorNote message={error} />}

      <div className="grid gap-4 @3xl:grid-cols-3">
        <Stat label="Total revenue" value={<Platinum value={revenue} />} />
        <Stat label="Total expenses" value={<Platinum value={expenses} />} />
        <Stat
          label="Profit"
          value={
            <Platinum
              value={revenue - expenses}
              tone={changeTone(revenue - expenses)}
            />
          }
        />
      </div>

      {data.categories.length === 0 ? (
        <EmptyPanel>
          No sales or purchases recorded in this window. Only trades of items
          for platinum count.
        </EmptyPanel>
      ) : (
        <>
          <Section
            title="By Category"
            description="A trade of several items counts under its set, or under its first item when the items are not one set."
          >
            <DataTable
              tableId="analyticsCategories"
              columns={CATEGORY_COLUMNS}
              data={data.categories}
              searchPlaceholder="Filter categories"
              initialSorting={[{ id: "profit", desc: true }]}
              rowKey={(row) => row.category}
            />
          </Section>

          <div className="grid gap-4 @5xl:grid-cols-2">
            <Section title="Top Sales">
              <DataTable
                tableId="analyticsSold"
                columns={TRADED_COLUMNS}
                data={data.sold}
                searchPlaceholder="Filter sales"
                initialSorting={[{ id: "value", desc: true }]}
                rowKey={(row) => row.name}
                initialPageSize={10}
                emptyMessage="Nothing sold in this window."
              />
            </Section>
            <Section title="Top Purchases">
              <DataTable
                tableId="analyticsBought"
                columns={TRADED_COLUMNS}
                data={data.bought}
                searchPlaceholder="Filter purchases"
                initialSorting={[{ id: "value", desc: true }]}
                rowKey={(row) => row.name}
                initialPageSize={10}
                emptyMessage="Nothing bought in this window."
              />
            </Section>
          </div>
        </>
      )}

      <TradesSection
        tableId="analyticsTrades"
        trades={data.trades}
        description="Every trade in this window, recorded from the game or by hand. Only trades of items for platinum feed the figures above."
      />
    </>
  );
}
