import { Filter, X } from "lucide-react";
import { type RefObject, useCallback, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { DataTable } from "@/components/data-table";
import { FilterField, FilterGrid } from "@/components/filter-grid";
import { FilterSelect, FilterSingleSelect } from "@/components/filter-select";
import {
  ErrorNote,
  Page,
  Quoted,
  Section,
  TableSkeleton,
} from "@/components/page";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useAsyncData } from "@/hooks/use-async-data";
import { useListen } from "@/hooks/use-listen";
import { api, events } from "@/lib/bridge";
import { type YesNo, yesNoOptions } from "@/lib/filters";
import { num, plat } from "@/lib/format";
import { usePageQuote } from "@/lib/quotes";
import {
  ANY_REFINEMENT,
  type FavouriteFilter,
  matchesRelicFilters,
  type RelicFilters,
  refinementValue,
  wantedChance,
  wantedKeysOf,
  writeOverlayFilters,
} from "@/lib/relic-filters";
import { REFINEMENTS, RELIC_TIERS } from "@/lib/relics";
import { usePreferencesStore } from "@/stores/preferences-store";
import {
  type PlannerRow,
  plannerColumns,
  rewardViews,
  searchValue,
} from "./columns";
import { PlannerRowDetails } from "./details";
import { PartSources, WantedParts } from "./parts";

const SQUAD_SIZES = ["1", "2", "3", "4"] as const;
const FAVOURITE_OPTIONS = [
  { value: "yes", label: "Favourites" },
  { value: "no", label: "Not favourites" },
  { value: "order", label: "Order first" },
];
const ALL_ERAS = "all";

export function RelicPlannerPage() {
  const quote = usePageQuote("relicPlanner");
  const { squadSize, refinement, era, setSquadSize, setRefinement, setEra } =
    usePreferencesStore();
  const [onlyOwned, setOnlyOwned] = useState(true);
  const [vaulted, setVaulted] = useState<YesNo | null>(null);
  const [itemsOwned, setItemsOwned] = useState<YesNo | null>(null);
  const [setsOwned, setSetsOwned] = useState<YesNo | null>(null);
  const [stacked, setStacked] = useState<YesNo | null>(null);
  const [refined, setRefined] = useState<YesNo | null>(null);
  const [tierOwned, setTierOwned] = useState<string | null>(null);
  const [favourite, setFavourite] = useState<FavouriteFilter | null>(null);
  const [wanted, setWanted] = useState<string[]>([]);
  const [selectedPart, setSelectedPart] = useState<string | null>(null);
  const wantedPanel = useRef<HTMLDivElement>(null);
  const sourcesPanel = useRef<HTMLDivElement>(null);

  const load = useCallback(
    () => api.relicPlannerTab(squadSize, onlyOwned),
    [squadSize, onlyOwned],
  );

  const { data, error, loading, reload } = useAsyncData(load);

  useListen(events.inventoryUpdated, reload);
  useListen(events.appReady, reload);

  const scrollToPanel = (panel: RefObject<HTMLDivElement | null>) => {
    panel.current?.scrollIntoView({ behavior: "smooth", block: "start" });
  };

  const missing = data?.missing_parts ?? [];

  const wantedKeys = useMemo(() => wantedKeysOf(wanted), [wanted]);

  const filters = useMemo<RelicFilters>(
    () => ({
      squadSize,
      refinement,
      favourite,
      vaulted,
      setsOwned,
      itemsOwned,
      stacked,
      refined,
      tierOwned,
      wanted,
    }),
    [
      squadSize,
      refinement,
      favourite,
      vaulted,
      setsOwned,
      itemsOwned,
      stacked,
      refined,
      tierOwned,
      wanted,
    ],
  );

  const handlePushToOverlay = useCallback(() => {
    writeOverlayFilters(filters);
    toast.success("The relic overlay now uses these filters");
  }, [filters]);

  const owned = data?.plans ?? [];

  const eras = useMemo(() => {
    const present = new Set(owned.map((plan) => plan.tier));
    return RELIC_TIERS.filter((tier) => present.has(tier));
  }, [owned]);

  const rows = useMemo(
    () =>
      owned
        .filter(
          (plan) =>
            (era === ALL_ERAS || plan.tier === era) &&
            matchesRelicFilters(plan, filters, wantedKeys),
        )
        .flatMap<PlannerRow>((plan) => {
          const best = refinement === ANY_REFINEMENT;
          const value = refinementValue(plan, refinement);
          if (!value) {
            return [];
          }
          return [
            {
              plan,
              value,
              rewards: rewardViews(plan, value, wantedKeys, squadSize),
              perTrace: best
                ? (plan.best.plat_per_trace?.value ?? null)
                : value.plat_per_trace,
              perTraceRefinement: best
                ? (plan.best.plat_per_trace?.refinement ?? null)
                : value.refinement,
              ducatsPerTrace: best
                ? (plan.best.ducats_per_trace?.value ?? null)
                : value.ducats_per_trace,
              ducatTraceRefinement: best
                ? (plan.best.ducats_per_trace?.refinement ?? null)
                : value.refinement,
              chanceOfWanted: wantedChance(plan, value, wantedKeys, squadSize),
            },
          ];
        }),
    [owned, era, filters, refinement, wantedKeys, squadSize],
  );

  const best = useMemo(
    () =>
      rows.reduce<PlannerRow | null>(
        (top, row) =>
          top === null || row.value.expected_plat > top.value.expected_plat
            ? row
            : top,
        null,
      ),
    [rows],
  );

  const columns = useMemo(
    () => plannerColumns(squadSize, wantedKeys),
    [squadSize, wantedKeys],
  );

  if (loading && !data) {
    return (
      <Page title="Relic Planner" description={<Quoted quote={quote} />}>
        <TableSkeleton />
      </Page>
    );
  }

  const activeFilters = [
    vaulted,
    itemsOwned,
    setsOwned,
    stacked,
    refined,
    tierOwned,
    favourite,
  ].filter((choice) => choice !== null).length;
  const traces = data?.void_traces ?? 0;
  const counted = `${num(rows.length)} ${rows.length === 1 ? "relic" : "relics"}`;
  const summary = best
    ? `${counted}, best is ${best.plan.relic} at ${plat(best.value.expected_plat)} platinum expected (${best.value.refinement}), ${num(traces)} Void Traces`
    : `No relic matches these filters, ${num(traces)} Void Traces`;

  return (
    <Page
      title="Relic Planner"
      description={<Quoted quote={quote} />}
      actions={
        <>
          <Button variant="outline" size="sm" onClick={handlePushToOverlay}>
            Push Filters to the Overlay
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => scrollToPanel(wantedPanel)}
          >
            Wanted Parts
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => scrollToPanel(sourcesPanel)}
          >
            Drops From
          </Button>
        </>
      }
    >
      {error && <ErrorNote message={error} />}

      <Section
        title={onlyOwned ? "Owned Relics" : "Every Relic"}
        description={summary}
        action={
          <Tabs value={era} onValueChange={setEra}>
            <TabsList>
              <TabsTrigger value={ALL_ERAS}>All</TabsTrigger>
              {eras.map((tier) => (
                <TabsTrigger key={tier} value={tier}>
                  {tier}
                </TabsTrigger>
              ))}
            </TabsList>
          </Tabs>
        }
      >
        <DataTable
          tableId="relicPlanner"
          columns={columns}
          data={rows}
          searchPlaceholder="Filter relics and rewards"
          searchValue={searchValue}
          initialSorting={[{ id: "expected_plat", desc: true }]}
          primarySort={
            favourite === "order"
              ? [{ id: "favourites", desc: true }]
              : undefined
          }
          rowKey={(row) => row.plan.relic}
          toolbar={
            wanted.length > 0 && (
              <Button variant="outline" size="sm" onClick={() => setWanted([])}>
                <Filter className="size-3.5" />
                {wanted.length} wanted parts
                <X className="size-3.5" />
              </Button>
            )
          }
          filters={
            <FilterGrid
              activeFilters={activeFilters}
              onClear={() => {
                setVaulted(null);
                setItemsOwned(null);
                setSetsOwned(null);
                setStacked(null);
                setRefined(null);
                setTierOwned(null);
                setFavourite(null);
              }}
            >
              <FilterSingleSelect
                label="Squad"
                value={String(squadSize)}
                options={SQUAD_SIZES.map((size) => ({
                  value: size,
                  label: size,
                }))}
                onChange={(size) => setSquadSize(Number(size))}
              />
              <FilterSingleSelect
                label="Refinement"
                value={refinement}
                options={[
                  { value: ANY_REFINEMENT, label: "Any" },
                  ...REFINEMENTS.map((entry) => ({
                    value: entry,
                    label: entry,
                  })),
                ]}
                onChange={setRefinement}
              />
              <FilterSelect
                label="Vault"
                value={vaulted}
                options={yesNoOptions("Vaulted", "Available")}
                onChange={(next) => setVaulted(next as YesNo | null)}
              />
              <FilterSelect
                label="Items owned"
                value={itemsOwned}
                options={yesNoOptions("All built or mastered", "Some missing")}
                onChange={(next) => setItemsOwned(next as YesNo | null)}
              />
              <FilterSelect
                label="Sets owned"
                value={setsOwned}
                options={yesNoOptions("All complete", "Incomplete")}
                onChange={(next) => setSetsOwned(next as YesNo | null)}
              />
              <FilterSelect
                label="Copies"
                value={stacked}
                options={yesNoOptions("10 or more", "Fewer than 10")}
                onChange={(next) => setStacked(next as YesNo | null)}
              />
              <FilterSelect
                label="Refined"
                value={refined}
                options={yesNoOptions("Refined copies", "Intact only")}
                onChange={(next) => setRefined(next as YesNo | null)}
              />
              <FilterSelect
                label="Tier owned"
                value={tierOwned}
                options={REFINEMENTS.map((entry) => ({
                  value: entry,
                  label: entry,
                }))}
                onChange={setTierOwned}
              />
              <FilterSelect
                label="Favourite"
                value={favourite}
                options={FAVOURITE_OPTIONS}
                onChange={(next) =>
                  setFavourite(next as FavouriteFilter | null)
                }
              />
              <FilterField label="Relics">
                <div className="flex h-8 items-center gap-2">
                  <Checkbox
                    id="onlyOwnedRelics"
                    checked={onlyOwned}
                    onCheckedChange={(value) => setOnlyOwned(value === true)}
                  />
                  <Label
                    htmlFor="onlyOwnedRelics"
                    className="text-xs font-normal"
                  >
                    Only owned
                  </Label>
                </div>
              </FilterField>
            </FilterGrid>
          }
          renderSubRow={(row) => <PlannerRowDetails row={row} />}
        />
      </Section>

      <div className="grid gap-4 lg:grid-cols-2">
        <div ref={wantedPanel} className="scroll-mt-6">
          <WantedParts
            missing={missing}
            wanted={wanted}
            setWanted={setWanted}
            selected={selectedPart}
            onSelect={setSelectedPart}
          />
        </div>

        <div ref={sourcesPanel} className="scroll-mt-6">
          <PartSources part={selectedPart} />
        </div>
      </div>
    </Page>
  );
}
