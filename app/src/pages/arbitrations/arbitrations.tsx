import { useState } from "react";
import { FilterGrid } from "@/components/filter-grid";
import { FilterSelect } from "@/components/filter-select";
import { OptionTabs } from "@/components/option-tabs";
import { Page, Section } from "@/components/page";
import { SearchInput } from "@/components/search-input";
import { useAsyncData } from "@/hooks/use-async-data";
import { useFilters } from "@/hooks/use-filters";
import { useNow } from "@/hooks/use-now";
import {
  ARBITRATION_RELOAD_EVENTS,
  ARBITRATION_TIERS,
  currentSlot,
  groupByDay,
  liveSlots,
  nextSlot,
  nodeValues,
  slotSearchText,
} from "@/lib/arbitrations";
import { api } from "@/lib/bridge";
import { type FilterOption, plainOptions } from "@/lib/filters";
import { counted } from "@/lib/format";
import type {
  ArbitrationDashboard,
  ArbitrationForecastStatus,
  ArbitrationTier,
} from "@/types";
import { ForecastNotice, ForecastSource } from "./panels";
import { DayList, HeroCards } from "./schedule";

const ALL_DAYS = "all";

const TIERS: FilterOption<ArbitrationTier>[] = ARBITRATION_TIERS.map(
  (tier) => ({ value: tier, label: tier }),
);

const STATUS_NOTE: Record<ArbitrationForecastStatus, string | null> = {
  fresh: null,
  cached: "The forecast could not be refreshed. This is the saved copy.",
  stale: "The saved forecast is older than a day.",
  unavailable: "The forecast is not available. Merframe retries on its own.",
};

function statusNote(dashboard: ArbitrationDashboard | null): string | null {
  return dashboard ? STATUS_NOTE[dashboard.status] : null;
}

export function ArbitrationsPage() {
  const { data, error, loading } = useAsyncData(
    "arbitrations",
    api.arbitrations,
    ARBITRATION_RELOAD_EVENTS,
  );
  const now = useNow();
  const [search, setSearch] = useState("");
  const [day, setDay] = useState(ALL_DAYS);
  const filters = useFilters<{
    mission: string | null;
    planet: string | null;
    faction: string | null;
    tier: ArbitrationTier | null;
  }>({
    mission: null,
    planet: null,
    faction: null,
    tier: null,
  });

  const schedule = data?.schedule ?? null;
  const notice = statusNote(data);
  const slots = liveSlots(schedule, now);
  const current = currentSlot(slots, now);
  const next = nextSlot(slots, now);
  const nodes = schedule?.nodes ?? [];

  const query = search.trim().toLocaleLowerCase();
  const chosen = filters.values;
  const shown = slots.filter(
    (slot) =>
      (!query || slotSearchText(slot).includes(query)) &&
      (chosen.mission === null || slot.mission_type === chosen.mission) &&
      (chosen.planet === null || slot.planet === chosen.planet) &&
      (chosen.faction === null || slot.faction === chosen.faction) &&
      (chosen.tier === null || slot.tier === chosen.tier),
  );
  const days = groupByDay(shown, now);
  const dayTabs = [
    { value: ALL_DAYS, label: "All days" },
    ...groupByDay(slots, now).map((day) => ({
      value: day.key,
      label: day.label,
    })),
  ];
  const shownDays = days.filter(
    (entry) => day === ALL_DAYS || entry.key === day,
  );

  const clear = () => {
    setSearch("");
    setDay(ALL_DAYS);
    filters.clear();
  };

  return (
    <Page
      title="Arbitrations"
      quote="arbitrations"
      loading={loading && data === null}
      error={error}
    >
      {notice && <ForecastNotice>{notice}</ForecastNotice>}
      <HeroCards current={current} next={next} now={now} />

      <Section
        className="min-w-0"
        title="Next Seven Days"
        description={counted(shown.length, "rotation")}
        action={
          <OptionTabs
            value={dayTabs.some((tab) => tab.value === day) ? day : ALL_DAYS}
            options={dayTabs}
            onChange={setDay}
          />
        }
      >
        <div className="flex flex-col gap-4">
          <SearchInput
            value={search}
            onValueChange={setSearch}
            placeholder="Search the schedule"
          />
          <FilterGrid activeFilters={filters.active} onClear={filters.clear}>
            <FilterSelect
              label="Mission type"
              value={chosen.mission}
              options={plainOptions(nodeValues(nodes, "mission_type"))}
              onChange={(value) => filters.set("mission", value)}
            />
            <FilterSelect
              label="Planet"
              value={chosen.planet}
              options={plainOptions(nodeValues(nodes, "planet"))}
              onChange={(value) => filters.set("planet", value)}
            />
            <FilterSelect
              label="Faction"
              value={chosen.faction}
              options={plainOptions(nodeValues(nodes, "faction"))}
              onChange={(value) => filters.set("faction", value)}
            />
            <FilterSelect
              label="Community tier"
              value={chosen.tier}
              options={TIERS}
              onChange={(value) => filters.set("tier", value)}
            />
          </FilterGrid>
          <DayList
            days={shownDays}
            total={slots.length}
            now={now}
            onClear={clear}
          />
        </div>
      </Section>

      <ForecastSource />
    </Page>
  );
}
