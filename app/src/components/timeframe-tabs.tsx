import { OptionTabs } from "@/components/option-tabs";
import { DAY_MS } from "@/lib/world";
import { usePreferencesStore } from "@/stores/preferences-store";
import type { Timeframe } from "@/types";

const TIMEFRAMES: { value: Timeframe; label: string }[] = [
  { value: "7", label: "7 days" },
  { value: "30", label: "30 days" },
  { value: "90", label: "90 days" },
  { value: "all", label: "All history" },
];

export function sinceMs(timeframe: Timeframe): number | undefined {
  if (timeframe === "all") {
    return undefined;
  }
  return Date.now() - Number(timeframe) * DAY_MS;
}

export function TimeframeTabs() {
  const { timeframe, setTimeframe } = usePreferencesStore();

  return (
    <OptionTabs
      value={timeframe}
      options={TIMEFRAMES}
      onChange={setTimeframe}
    />
  );
}
