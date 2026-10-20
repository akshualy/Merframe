import { useCallback } from "react";
import { OptionTabs } from "@/components/option-tabs";
import { useAsyncData } from "@/hooks/use-async-data";
import type { EventName } from "@/lib/bridge";
import { DAY_MS } from "@/lib/world";
import { usePreferencesStore } from "@/stores/preferences-store";
import type { Timeframe, TimeframeCard } from "@/types";

const TIMEFRAMES: { value: Timeframe; label: string }[] = [
  { value: "today", label: "Today" },
  { value: "7", label: "7d" },
  { value: "30", label: "30d" },
  { value: "90", label: "90d" },
  { value: "all", label: "All" },
];

function sinceMs(timeframe: Timeframe): number | undefined {
  if (timeframe === "all") {
    return undefined;
  }
  if (timeframe === "today") {
    return new Date().setHours(0, 0, 0, 0);
  }
  return Date.now() - Number(timeframe) * DAY_MS;
}

function useTimeframe(card: TimeframeCard): Timeframe {
  return usePreferencesStore((state) => state.timeframes[card]);
}

export function useTimeframeData<T>(
  card: TimeframeCard,
  fetch: (sinceMs?: number) => Promise<T>,
  reloadOn: readonly EventName[],
) {
  const timeframe = useTimeframe(card);
  const load = useCallback(() => fetch(sinceMs(timeframe)), [fetch, timeframe]);
  return useAsyncData(`${card}:${timeframe}`, load, reloadOn);
}

export function TimeframeTabs({ card }: { card: TimeframeCard }) {
  const timeframe = useTimeframe(card);
  const setTimeframe = usePreferencesStore((state) => state.setTimeframe);

  return (
    <OptionTabs
      value={timeframe}
      options={TIMEFRAMES}
      onChange={(next) => setTimeframe(card, next)}
    />
  );
}
