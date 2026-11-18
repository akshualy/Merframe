import { ArrowDownWideNarrow, ArrowUpNarrowWide } from "lucide-react";
import { useEffect } from "react";
import { FilterGrid } from "@/components/filter-grid";
import { FilterSelect } from "@/components/filter-select";
import { ItemCard } from "@/components/inventory-item-card";
import { prefetchImages } from "@/components/item-image";
import { OptionSelect } from "@/components/option-select";
import { CardGrid, EmptyPanel } from "@/components/page";
import { Pagination } from "@/components/pagination";
import { SearchInput } from "@/components/search-input";
import { Button } from "@/components/ui/button";
import { usePaged } from "@/hooks/use-paged";
import {
  NO_FILTER_MATCHES,
  plainOptions,
  withYesNo,
  yesNoOptions,
} from "@/lib/filters";
import { num } from "@/lib/format";
import {
  entryKey,
  type InventoryEntry,
  type InventoryFlags,
  type InventoryTabKey,
} from "@/lib/inventory-entries";
import {
  descending,
  ORDERINGS,
  refinementsFor,
  type TabFilters,
  yesNoFiltersFor,
} from "@/lib/inventory-filters";

const MIN_PLAT_CHOICES = plainOptions(["5", "10", "15"]);

export function InventoryTabView({
  tab,
  entryCount,
  visible,
  unpriced,
  flags,
  filters,
  updateFilters,
}: {
  tab: InventoryTabKey;
  entryCount: number;
  visible: InventoryEntry[];
  unpriced: number;
  flags: InventoryFlags;
  filters: TabFilters;
  updateFilters: (patch: Partial<TabFilters>) => void;
}) {
  const { pageItems: pageEntries, pagination } = usePaged(
    visible,
    JSON.stringify(filters),
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
          onValueChange={(search) => updateFilters({ search })}
          placeholder="Search"
        />

        <OptionSelect
          label="Order by"
          value={filters.ordering}
          options={ORDERINGS}
          onChange={(ordering) => updateFilters({ ordering })}
          className="w-44"
        />

        <Button
          variant="outline"
          size="icon"
          aria-label={descending(filters) ? "Largest first" : "Smallest first"}
          title={descending(filters) ? "Largest first" : "Smallest first"}
          onClick={() =>
            updateFilters({ preferHighest: !filters.preferHighest })
          }
        >
          {descending(filters) ? (
            <ArrowDownWideNarrow className="size-4" />
          ) : (
            <ArrowUpNarrowWide className="size-4" />
          )}
        </Button>

        <span className="text-muted-foreground ml-auto text-xs">
          {num(visible.length)} of {num(entryCount)} shown
          {unpriced > 0 ? `, ${num(unpriced)} without a price yet` : ""}
        </span>
      </div>

      <FilterGrid
        activeFilters={activeFilters}
        onClear={() =>
          updateFilters({ yesNo: {}, minPlat: null, refinement: null })
        }
      >
        {yesNoFiltersFor(tab).map((group) => (
          <FilterSelect
            key={group.key}
            label={group.label}
            value={filters.yesNo[group.key] ?? null}
            options={yesNoOptions(group.yes, group.no)}
            onChange={(mode) =>
              updateFilters({
                yesNo: withYesNo(filters.yesNo, group.key, mode),
              })
            }
          />
        ))}
        {refinements.length > 0 && (
          <FilterSelect
            label="Refinement"
            value={filters.refinement}
            options={plainOptions(refinements)}
            onChange={(value) =>
              updateFilters({
                refinement: value,
              })
            }
          />
        )}
        <FilterSelect
          label="Min plat"
          value={filters.minPlat === null ? null : String(filters.minPlat)}
          options={MIN_PLAT_CHOICES}
          onChange={(value) =>
            updateFilters({ minPlat: value === null ? null : Number(value) })
          }
        />
      </FilterGrid>

      {pageEntries.length === 0 ? (
        <EmptyPanel>{NO_FILTER_MATCHES}</EmptyPanel>
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
    </div>
  );
}
