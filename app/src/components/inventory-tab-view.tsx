import { ArrowDownWideNarrow, ArrowUpNarrowWide } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { FilterGrid } from "@/components/filter-grid";
import { FilterSelect } from "@/components/filter-select";
import { GameIcon } from "@/components/game-icon";
import { ItemCard } from "@/components/inventory-item-card";
import { prefetchImages } from "@/components/item-image";
import { CardGrid, EmptyPanel } from "@/components/page";
import { SearchInput } from "@/components/search-input";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { type YesNo, yesNoOptions } from "@/lib/filters";
import { num } from "@/lib/format";
import {
  compareRows,
  descending,
  type InventoryTabKey,
  loadFilters,
  ORDERINGS,
  type Ordering,
  refinementsFor,
  saveFilters,
  type TabFilters,
  yesNoFiltersFor,
} from "@/lib/inventory-filters";
import { keeps, type Row, rowPlat } from "@/lib/inventory-rows";
import { useAppStore } from "@/stores/app-store";

const MIN_PLAT_CHOICES = [5, 10, 15];

export function InventoryTabView({
  tab,
  rows,
}: {
  tab: InventoryTabKey;
  rows: Row[];
}) {
  const [filters, setFilters] = useState<TabFilters>(() => loadFilters(tab));
  const { settings } = useAppStore();
  const showEveryRow = settings?.show_full_inventory ?? false;
  const [capLifted, setCapLifted] = useState(false);

  const update = (patch: Partial<TabFilters>) => {
    setFilters((current) => {
      const next = { ...current, ...patch };
      saveFilters(tab, next);
      return next;
    });
  };

  const visible = useMemo(() => {
    const desc = descending(filters);
    return rows
      .filter((row) => keeps(row, filters))
      .sort((left, right) =>
        compareRows(left, right, filters.ordering, desc, tab),
      );
  }, [rows, filters, tab]);

  const capped = !showEveryRow && !capLifted;
  const cappedRows = useMemo(
    () => (capped ? visible.slice(0, 300) : visible),
    [visible, capped],
  );

  const selection = useMemo(
    () =>
      visible.reduce(
        (accumulator, row) => ({
          ducats: accumulator.ducats + (row.ducats ?? 0) * row.count,
          plat: accumulator.plat + (rowPlat(row) ?? 0) * row.count,
          priced: accumulator.priced + (rowPlat(row) === null ? 0 : 1),
        }),
        { ducats: 0, plat: 0, priced: 0 },
      ),
    [visible],
  );

  useEffect(() => {
    prefetchImages([
      ...cappedRows.map((row) => row.imageName),
      ...cappedRows.flatMap(
        (row) => row.set?.components.map((part) => part.image_name) ?? [],
      ),
    ]);
  }, [cappedRows]);

  const refinements = refinementsFor(tab);
  const activeFilters =
    Object.keys(filters.yesNo).length +
    (filters.minPlat === null ? 0 : 1) +
    (filters.refinement === null ? 0 : 1);

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <SearchInput
          value={filters.search}
          onChange={(e) => update({ search: e.target.value })}
          placeholder="Search"
        />

        <Select
          value={filters.ordering}
          onValueChange={(value) => update({ ordering: value as Ordering })}
        >
          <SelectTrigger className="w-44" aria-label="Order by">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {ORDERINGS.map((option) => (
              <SelectItem key={option.value} value={option.value}>
                {option.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        <Button
          variant="outline"
          size="icon"
          title={descending(filters) ? "Largest first" : "Smallest first"}
          onClick={() => update({ preferHighest: !filters.preferHighest })}
        >
          {descending(filters) ? (
            <ArrowDownWideNarrow className="size-4" />
          ) : (
            <ArrowUpNarrowWide className="size-4" />
          )}
        </Button>

        <span className="text-muted-foreground ml-auto text-xs">
          {num(visible.length)} of {num(rows.length)} shown
          {selection.priced < visible.length
            ? `, ${num(visible.length - selection.priced)} without a price yet`
            : ""}
        </span>
      </div>

      <FilterGrid
        activeFilters={activeFilters}
        onClear={() => update({ yesNo: {}, minPlat: null, refinement: null })}
      >
        {yesNoFiltersFor(tab).map((group) => (
          <FilterSelect
            key={group.key}
            label={group.label}
            value={filters.yesNo[group.key] ?? null}
            options={yesNoOptions(group.yes, group.no)}
            onChange={(mode) => {
              const next = { ...filters.yesNo };
              if (mode === null) {
                delete next[group.key];
              } else {
                next[group.key] = mode as YesNo;
              }
              update({ yesNo: next });
            }}
          />
        ))}
        {refinements.length > 0 && (
          <FilterSelect
            label="Refinement"
            value={filters.refinement}
            options={refinements.map((value) => ({ value, label: value }))}
            onChange={(value) => update({ refinement: value })}
          />
        )}
        <FilterSelect
          label="Min plat"
          value={filters.minPlat === null ? null : String(filters.minPlat)}
          options={MIN_PLAT_CHOICES.map((value) => ({
            value: String(value),
            label: String(value),
          }))}
          onChange={(value) =>
            update({ minPlat: value === null ? null : Number(value) })
          }
        />
      </FilterGrid>

      {cappedRows.length === 0 ? (
        <EmptyPanel>Nothing matches these filters.</EmptyPanel>
      ) : (
        <CardGrid>
          {cappedRows.map((row) => (
            <ItemCard key={row.key} row={row} tab={tab} />
          ))}
        </CardGrid>
      )}

      {visible.length > cappedRows.length && (
        <div className="flex flex-wrap items-center gap-3">
          <Hint>
            Showing the first {num(cappedRows.length)} of {num(visible.length)}.
            Search or filter to narrow it down.
          </Hint>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setCapLifted(true)}
          >
            Show All
          </Button>
        </div>
      )}

      <div className="text-muted-foreground flex items-center justify-end gap-6 text-sm">
        <span
          className="flex items-center gap-1"
          title="Total of the rows shown"
        >
          Ducats
          <span className="text-foreground flex items-center gap-0.5 font-semibold tabular-nums">
            {num(selection.ducats)}
            <GameIcon name="ducats" size={16} alt="Ducats" />
          </span>
        </span>
        <span
          className="flex items-center gap-1"
          title="Total of the rows shown"
        >
          Platinum
          <span className="text-primary flex items-center gap-0.5 font-semibold tabular-nums">
            {num(selection.plat)}
            <GameIcon name="platinum" size={16} alt="Platinum" />
          </span>
        </span>
      </div>
    </div>
  );
}
