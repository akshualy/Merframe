import type { ColumnDef } from "@tanstack/react-table";
import { X } from "lucide-react";
import { useCallback, useMemo } from "react";
import { type Bar, BarChart } from "@/components/bar-chart";
import { Amount } from "@/components/currency";
import { DataTable } from "@/components/data-table";
import { type Day, DayStrip } from "@/components/day-strip";
import { Figure, Figures } from "@/components/figure";
import { ItemSearch } from "@/components/item-search";
import { dayPoints, LineChart, type Point } from "@/components/line-chart";
import { Page, Section } from "@/components/page";
import { TimeframeTabs, useTimeframeData } from "@/components/timeframe-tabs";
import { TradesSection } from "@/components/trades-section";
import { Button } from "@/components/ui/button";
import { useAsyncData } from "@/hooks/use-async-data";
import { api } from "@/lib/bridge";
import { changeTone, signed } from "@/lib/chart";
import {
  counted,
  dateTime,
  dayLabel,
  fullDate,
  num,
  spansYears,
} from "@/lib/format";
import { cn } from "@/lib/utils";
import { usePreferencesStore } from "@/stores/preferences-store";
import type { DailyCount, DeltaRow, OpeningRow, StatPoint } from "@/types";

const OPENING_COLUMNS: ColumnDef<OpeningRow>[] = [
  {
    accessorKey: "at",
    header: "When",
    cell: ({ row }) => dateTime(row.original.at),
  },
  { accessorKey: "relic", header: "Relic", enableHiding: false },
  { accessorKey: "reward", header: "Reward", meta: { wrap: true } },
  {
    accessorKey: "player_count",
    header: "Squad",
    meta: { numeric: true, label: "Squad size" },
    cell: ({ row }) => num(row.original.player_count),
  },
];

const DELTA_COLUMNS: ColumnDef<DeltaRow>[] = [
  { accessorKey: "name", header: "Item", enableHiding: false },
  { accessorKey: "category", header: "Category" },
  {
    accessorKey: "delta",
    header: "Change",
    meta: { numeric: true },
    cell: ({ row }) => (
      <span className={cn("font-medium", changeTone(row.original.delta))}>
        {signed(row.original.delta, num)}
      </span>
    ),
  },
];

const HISTORY_COLOR_SLOTS = [1, 2, 3, 4, 5, 6];

export function StatsPage() {
  const { data, error, loading } = useTimeframeData("history", api.statsTab, [
    "inventoryUpdated",
    "tradesUpdated",
  ]);
  const deltas = useTimeframeData("changes", api.statsDeltas, [
    "inventoryUpdated",
  ]);
  const openings = useTimeframeData("relics", api.statsOpenings, [
    "inventoryUpdated",
  ]);
  const trades = useTimeframeData("trades", api.statsTrades, ["tradesUpdated"]);
  const historyItems = usePreferencesStore((state) => state.historyItems);
  const setHistoryItems = usePreferencesStore((state) => state.setHistoryItems);
  const loadItemHistory = useCallback(
    (sinceMs?: number) =>
      api.statsItemHistory(
        historyItems.map((chosen) => chosen.itemType),
        sinceMs,
      ),
    [historyItems],
  );
  const itemHistory = useTimeframeData("items", loadItemHistory, [
    "inventoryUpdated",
  ]);
  const itemChoices = useAsyncData("statsItemChoices", api.statsItemChoices, [
    "inventoryUpdated",
  ]);

  const points = data?.series ?? [];
  const summary = data?.summary ?? null;
  const days = data?.days_played ?? [];
  const withYear = spansYears(days.map((entry) => entry.day));

  const metrics = useMemo(() => {
    const byDay = new Map(
      points.map((point) => [point.at.slice(0, 10), point]),
    );
    const toPoints = (pick: (point: StatPoint) => number): Point[] =>
      dayPoints(
        days.map((entry) => {
          const point = byDay.get(entry.day);
          return { day: entry.day, value: point ? pick(point) : null };
        }),
        withYear,
      );
    return [
      {
        key: "plat",
        label: "Platinum",
        color: "var(--chart-1)",
        points: toPoints((point) => point.plat),
      },
      {
        key: "endo",
        label: "Endo",
        color: "var(--chart-3)",
        points: toPoints((point) => point.endo),
      },
      {
        key: "credits",
        label: "Credits",
        color: "var(--chart-4)",
        points: toPoints((point) => point.credits),
      },
    ];
  }, [points, days, withYear]);

  const dayBars = useMemo(() => {
    const toBars = (entries: DailyCount[]): Bar[] =>
      entries.map((entry) => ({
        key: entry.day,
        label: dayLabel(entry.day, withYear),
        value: entry.count,
      }));
    return {
      relics: toBars(data?.relics_per_day ?? []),
      trades: toBars(data?.trades_per_day ?? []),
    };
  }, [data?.relics_per_day, data?.trades_per_day, withYear]);

  const playedDayCells = useMemo<Day[]>(
    () =>
      days.map((entry) => ({
        key: entry.day,
        label: dayLabel(entry.day, withYear),
        played: entry.count > 0,
      })),
    [days, withYear],
  );

  const itemCharts = useMemo(() => {
    const histories = itemHistory.data ?? [];
    const itemsWithYear = spansYears(
      histories.at(0)?.counts.map((entry) => entry.day) ?? [],
    );
    return historyItems.flatMap((chosen) => {
      const history = histories.find(
        (entry) => entry.item.unique_name === chosen.itemType,
      );
      if (!history) {
        return [];
      }

      return {
        ...chosen,
        name: history.item.name,
        points: dayPoints(
          history.counts.map((entry) => ({
            day: entry.day,
            value: entry.count,
          })),
          itemsWithYear,
        ),
      };
    });
  }, [itemHistory.data, historyItems]);
  const freeColorSlot = HISTORY_COLOR_SLOTS.find(
    (slot) => !historyItems.some((chosen) => chosen.colorSlot === slot),
  );
  const addableItems = (itemChoices.data ?? []).filter(
    (item) =>
      !historyItems.some((chosen) => chosen.itemType === item.unique_name),
  );

  const firstDay = days.at(0);
  const lastDay = days.at(-1);
  const windowLabel =
    firstDay && lastDay
      ? `${dayLabel(firstDay.day, true)} to ${dayLabel(lastDay.day, true)}`
      : "nothing recorded in this window";
  const playedDays = days.filter((entry) => entry.count > 0).length;
  const latestRank = points.at(-1)?.mr ?? null;

  return (
    <Page
      title="Stats"
      quote="stats"
      actions={
        <Figures>
          <Figure label="Mastery rank" value={num(latestRank)} />
          <Figure
            label="Account created"
            value={summary ? fullDate(summary.account_created) : "-"}
          />
        </Figures>
      }
      loading={loading && !data}
      error={
        error ??
        itemHistory.error ??
        itemChoices.error ??
        deltas.error ??
        openings.error ??
        trades.error
      }
    >
      <Section
        title="History"
        description={`${num(playedDays)} out of ${counted(days.length, "day")} ${playedDays === 1 ? "has" : "have"} data. ${windowLabel}.`}
        action={
          <Figures>
            <Figure
              label="Platinum traded"
              value={
                <Amount currency="platinum" plain>
                  {num(data?.trade_plat)}
                </Amount>
              }
            />
            <TimeframeTabs card="history" />
          </Figures>
        }
      >
        <div className="grid gap-6 @lg:grid-cols-2 @5xl:grid-cols-3">
          {metrics.map((metric) => (
            <LineChart
              key={metric.key}
              label={metric.label}
              color={metric.color}
              points={metric.points}
              format={num}
            />
          ))}
          <BarChart
            label="Relics opened"
            color="var(--chart-6)"
            bars={dayBars.relics}
            format={num}
          />
          <BarChart
            label="Daily trades"
            color="var(--chart-6)"
            bars={dayBars.trades}
            format={num}
          />
          <DayStrip
            label="Days played"
            color="var(--chart-6)"
            days={playedDayCells}
            format={num}
          />
        </div>
      </Section>

      <Section
        title="Item History"
        description={`Pick up to ${HISTORY_COLOR_SLOTS.length} items.`}
        action={<TimeframeTabs card="items" />}
        controls={
          freeColorSlot !== undefined && (
            <ItemSearch
              items={addableItems}
              placeholder="Add an item"
              onPick={(item) =>
                setHistoryItems([
                  ...historyItems,
                  { itemType: item.unique_name, colorSlot: freeColorSlot },
                ])
              }
            />
          )
        }
      >
        {historyItems.length === 0 ? (
          <p className="text-muted-foreground text-sm">
            No items picked. Add one above.
          </p>
        ) : (
          <div className="grid gap-6 @lg:grid-cols-2 @5xl:grid-cols-3">
            {itemCharts.map((chart) => (
              <LineChart
                key={chart.itemType}
                label={chart.name}
                color={`var(--chart-${chart.colorSlot})`}
                points={chart.points}
                format={num}
                control={
                  <Button
                    variant="ghost"
                    size="icon"
                    className="size-6"
                    aria-label={`Remove ${chart.name}`}
                    onClick={() =>
                      setHistoryItems(
                        historyItems.filter(
                          (chosen) => chosen.itemType !== chart.itemType,
                        ),
                      )
                    }
                  >
                    <X className="size-4" />
                  </Button>
                }
              />
            ))}
          </div>
        )}
      </Section>

      <div className="grid gap-4 @5xl:grid-cols-2">
        <Section
          title="Inventory Changes"
          action={<TimeframeTabs card="changes" />}
        >
          <DataTable
            tableId="statsDeltas"
            columns={DELTA_COLUMNS}
            data={deltas.data ?? []}
            searchPlaceholder="Filter changes"
            initialSorting={[{ id: "delta", desc: true }]}
            rowKey={(row) => row.item_type}
            emptyMessage="No inventory changes in this window."
          />
        </Section>
        <Section
          title="Relic Openings"
          description="Shows what your own relic dropped, not the reward you picked."
          action={<TimeframeTabs card="relics" />}
        >
          <DataTable
            tableId="statsOpenings"
            columns={OPENING_COLUMNS}
            data={openings.data ?? []}
            searchPlaceholder="Filter openings"
            initialSorting={[{ id: "at", desc: true }]}
            rowKey={(row) => String(row.id)}
            emptyMessage="No relic openings in this window."
          />
        </Section>
      </div>

      <TradesSection
        tableId="statsTrades"
        trades={trades.data ?? []}
        filter={<TimeframeTabs card="trades" />}
      />
    </Page>
  );
}
