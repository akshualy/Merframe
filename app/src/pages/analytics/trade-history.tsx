import type { ColumnDef } from "@tanstack/react-table";
import { Amount } from "@/components/currency";
import { DataTable } from "@/components/data-table";
import {
  EmptyPanel,
  ErrorNote,
  Section,
  Stat,
  TableSkeleton,
} from "@/components/page";
import { TimeframeTabs, useTimeframeData } from "@/components/timeframe-tabs";
import { TradesSection } from "@/components/trades-section";
import { api } from "@/lib/bridge";
import { changeTone, signed } from "@/lib/chart";
import { num } from "@/lib/format";
import { cn } from "@/lib/utils";
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
          {signed(profit, num)}
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

export function TradeHistory() {
  const { data, error } = useTimeframeData("tradeHistory", api.tradeAnalytics, [
    "tradesUpdated",
  ]);

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
  const totals = [
    { label: "Total revenue", value: revenue },
    { label: "Total expenses", value: expenses },
    {
      label: "Profit",
      value: revenue - expenses,
      className: changeTone(revenue - expenses),
    },
  ];

  return (
    <>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2 className="text-foreground text-lg font-bold">Your Trades</h2>
        <TimeframeTabs card="tradeHistory" />
      </div>

      {error && <ErrorNote message={error} />}

      <div className="grid gap-4 @3xl:grid-cols-3">
        {totals.map((total) => (
          <Stat
            key={total.label}
            label={total.label}
            value={
              <Amount currency="platinum" plain className={total.className}>
                {num(total.value)}
              </Amount>
            }
          />
        ))}
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
