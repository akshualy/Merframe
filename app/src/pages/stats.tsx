import type { ColumnDef } from "@tanstack/react-table";
import { useCallback, useMemo } from "react";
import { type Bar, BarChart } from "@/components/bar-chart";
import { DataTable } from "@/components/data-table";
import { type Day, DayStrip } from "@/components/day-strip";
import { LineChart, type Point } from "@/components/line-chart";
import {
  ErrorNote,
  Page,
  Quoted,
  Section,
  Stat,
  TableSkeleton,
} from "@/components/page";
import { sinceMs, TimeframeTabs } from "@/components/timeframe-tabs";
import { TradesSection } from "@/components/trades-section";
import { useAsyncData } from "@/hooks/use-async-data";
import { api } from "@/lib/bridge";
import { changeTone } from "@/lib/chart";
import { dateTime, dayLabel, num, percent, spansYears } from "@/lib/format";
import { usePageQuote } from "@/lib/quotes";
import { cn } from "@/lib/utils";
import { usePreferencesStore } from "@/stores/preferences-store";
import type { DailyCount, DeltaRow, OpeningRow, StatPoint } from "@/types";

function fullDate(iso: string): string {
  return new Date(iso).toLocaleDateString("en-GB", {
    day: "2-digit",
    month: "short",
    year: "numeric",
  });
}

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
        {row.original.delta > 0 ? "+" : ""}
        {num(row.original.delta)}
      </span>
    ),
  },
];

export function StatsPage() {
  const quote = usePageQuote("stats");
  const { timeframe } = usePreferencesStore();
  const load = useCallback(() => api.statsTab(sinceMs(timeframe)), [timeframe]);
  const { data, error, loading } = useAsyncData(load, ["inventoryUpdated"]);

  const points = data?.series ?? [];
  const summary = data?.summary ?? null;
  const days = data?.days_played ?? [];
  const withYear = spansYears(days.map((entry) => entry.day));

  const metrics = useMemo(() => {
    const byDay = new Map(
      points.map((point) => [point.at.slice(0, 10), point]),
    );
    const toPoints = (pick: (point: StatPoint) => number | null): Point[] => {
      let previous: number | null = null;
      return days.map((entry) => {
        const point = byDay.get(entry.day);
        const value = point ? pick(point) : null;
        const change =
          value !== null && previous !== null ? value - previous : null;
        if (value !== null) {
          previous = value;
        }
        return {
          key: entry.day,
          label: dayLabel(entry.day, withYear),
          value,
          change,
        };
      });
    };
    const series = [
      {
        key: "plat",
        label: "Platinum",
        color: "var(--chart-1)",
        points: toPoints((point) => point.plat),
      },
      {
        key: "ducats",
        label: "Ducats",
        color: "var(--chart-2)",
        points: toPoints((point) => point.ducats),
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
      {
        key: "aya",
        label: "Aya",
        color: "var(--chart-5)",
        points: toPoints((point) => point.aya),
      },
    ];
    return series.filter(
      (entry) =>
        entry.key !== "aya" ||
        entry.points.some((point) => point.value !== null),
    );
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

  const tradePlat = (data?.trades ?? []).reduce(
    (sum, entry) => sum + entry.trade.plat,
    0,
  );
  const bestRelicDay = Math.max(0, ...dayBars.relics.map((bar) => bar.value));

  if (loading && !data) {
    return (
      <Page
        title="Stats"
        description={<Quoted quote={quote} />}
        actions={<TimeframeTabs />}
      >
        <TableSkeleton />
      </Page>
    );
  }

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
      description={<Quoted quote={quote} />}
      actions={<TimeframeTabs />}
    >
      {error && <ErrorNote message={error} />}

      <div className="grid gap-4 @sm:grid-cols-2 @3xl:grid-cols-4">
        <Stat
          label="Prime collection"
          value={summary ? percent(summary.prime_percent / 100, 0) : "-"}
          hint={
            summary
              ? `${num(summary.prime_owned)} of ${num(summary.prime_total)} Prime items owned or mastered`
              : "Waiting for an inventory read"
          }
        />
        <Stat
          label="Mastery rank"
          value={num(latestRank)}
          hint={
            summary
              ? `Account created ${fullDate(summary.account_created)}`
              : undefined
          }
        />
        <Stat
          label="Relics opened"
          value={num(data?.relic_openings.length ?? 0)}
          hint={`Best day ${num(bestRelicDay)}`}
        />
        <Stat
          label="Trades"
          value={num(data?.trades.length ?? 0)}
          hint={`${num(tradePlat)} platinum across them`}
        />
      </div>

      <Section
        title="History"
        description={`One point per UTC day. ${num(playedDays)} of ${num(days.length)} days have data, ${windowLabel}.`}
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

      <div className="grid gap-4 @5xl:grid-cols-2">
        <Section title="Latest Delta">
          <DataTable
            tableId="statsDeltas"
            columns={DELTA_COLUMNS}
            data={data?.latest_deltas ?? []}
            searchPlaceholder="Filter changes"
            initialSorting={[{ id: "delta", desc: true }]}
            rowKey={(row) => row.item_type}
            emptyMessage="No changes since the previous snapshot."
          />
        </Section>
        <Section
          title="Relic Openings"
          description="Shows what your own relic dropped, not the reward you picked."
        >
          <DataTable
            tableId="statsOpenings"
            columns={OPENING_COLUMNS}
            data={data?.relic_openings ?? []}
            searchPlaceholder="Filter openings"
            initialSorting={[{ id: "at", desc: true }]}
            rowKey={(row) => String(row.id)}
            emptyMessage="No relic openings recorded yet."
          />
        </Section>
      </div>

      <TradesSection tableId="statsTrades" trades={data?.trades ?? []} />
    </Page>
  );
}
