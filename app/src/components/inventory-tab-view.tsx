import { ArrowDownWideNarrow, ArrowUpNarrowWide } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { FilterGrid } from "@/components/filter-grid";
import { FilterSelect } from "@/components/filter-select";
import { GameIcon } from "@/components/game-icon";
import { ItemCard } from "@/components/inventory-item-card";
import { prefetchImages } from "@/components/item-image";
import { CardGrid, EmptyPanel } from "@/components/page";
import { Pagination } from "@/components/pagination";
import { SearchInput } from "@/components/search-input";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { usePaged } from "@/hooks/use-paged";
import { type YesNo, yesNoOptions } from "@/lib/filters";
import { num } from "@/lib/format";
import {
  ducats,
  entryKey,
  type InventoryEntry,
  type InventoryFlags,
  totalPlat,
} from "@/lib/inventory-entries";
import {
  compareEntries,
  descending,
  type InventoryTabKey,
  keeps,
  loadFilters,
  ORDERINGS,
  type Ordering,
  refinementsFor,
  saveFilters,
  type TabFilters,
  yesNoFiltersFor,
} from "@/lib/inventory-filters";

const MIN_PLAT_CHOICES = [5, 10, 15];

export function InventoryTabView({
  tab,
  entries,
  flags,
}: {
  tab: InventoryTabKey;
  entries: InventoryEntry[];
  flags: InventoryFlags;
}) {
  const [filters, setFilters] = useState<TabFilters>(() => loadFilters(tab));

  const update = (patch: Partial<TabFilters>) => {
    setFilters((current) => {
      const next = { ...current, ...patch };
      saveFilters(tab, next);
      return next;
    });
  };

  const visible = useMemo(() => {
    const desc = descending(filters);
    return entries
      .filter((entry) => keeps(entry, filters, flags))
      .sort((left, right) =>
        compareEntries(left, right, filters.ordering, desc, tab),
      );
  }, [entries, filters, flags, tab]);

  const { pageItems: pageEntries, pagination } = usePaged(
    visible,
    JSON.stringify(filters),
  );

  const selection = useMemo(
    () =>
      visible.reduce(
        (accumulator, entry) => ({
          ducats: accumulator.ducats + (ducats(entry) ?? 0) * entry.row.count,
          plat: accumulator.plat + (totalPlat(entry) ?? 0) * entry.row.count,
          priced: accumulator.priced + (totalPlat(entry) === null ? 0 : 1),
        }),
        { ducats: 0, plat: 0, priced: 0 },
      ),
    [visible],
  );

  useEffect(() => {
    prefetchImages([
      ...pageEntries.map((entry) => entry.item.image_name),
      ...pageEntries.flatMap((entry) =>
        entry.kind === "set"
          ? entry.parts.map((part) => part.item.image_name)
          : [],
      ),
    ]);
  }, [pageEntries]);

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
          onValueChange={(search) => update({ search })}
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
          {num(visible.length)} of {num(entries.length)} shown
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

      {pageEntries.length === 0 ? (
        <EmptyPanel>Nothing matches these filters.</EmptyPanel>
      ) : (
        <CardGrid className="@sm:grid-cols-1 @2xl:grid-cols-2 @6xl:grid-cols-3 @7xl:grid-cols-3">
          {pageEntries.map((entry) => (
            <ItemCard
              key={entryKey(entry)}
              entry={entry}
              tab={tab}
              flags={flags}
            />
          ))}
        </CardGrid>
      )}

      <Pagination {...pagination} />

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
