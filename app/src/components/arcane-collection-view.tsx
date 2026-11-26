import { memo, useEffect, useMemo, useState } from "react";
import { Amount } from "@/components/currency";
import { FavouriteStar } from "@/components/favourite-star";
import { FilterGrid } from "@/components/filter-grid";
import { FilterSelect } from "@/components/filter-select";
import { ArcaneImage, prefetchImages } from "@/components/item-image";
import { OptionSelect } from "@/components/option-select";
import { CardGrid, EmptyPanel } from "@/components/page";
import { Pagination } from "@/components/pagination";
import { RankedPlat } from "@/components/ranked-plat";
import { SearchInput } from "@/components/search-input";
import { Hint } from "@/components/ui/hint";
import { Progress } from "@/components/ui/progress";
import { useFilters } from "@/hooks/use-filters";
import { usePaged } from "@/hooks/use-paged";
import {
  collator,
  failsYesNo,
  matchesText,
  NO_FILTER_MATCHES,
  type YesNo,
  yesNoOptions,
} from "@/lib/filters";
import { num } from "@/lib/format";
import {
  type ArcaneCollectionEntry,
  arcaneCollectionEntries,
  missingCopies,
} from "@/lib/inventory-entries";
import { cn } from "@/lib/utils";
import { useMarketPanelStore } from "@/stores/market-panel-store";
import type {
  ArcaneCollectionRow,
  ArcaneCollectionStatus,
  InventoryTab,
} from "@/types";

export const ARCANE_STATUS_LABELS: Record<ArcaneCollectionStatus, string> = {
  collecting: "Collecting",
  can_be_maxed: "Can be maxed",
  maxed: "Maxed",
  not_owned: "Not owned",
};

const ARCANE_STATUS_OPTIONS = (
  ["collecting", "can_be_maxed", "maxed", "not_owned"] as const
).map((value) => ({ value, label: ARCANE_STATUS_LABELS[value] }));
const FAVOURITE = yesNoOptions("Favourites", "Not favourites");

const ARCANE_COLLECTION_ORDERINGS = [
  { value: "missing", label: "Fewest missing" },
  { value: "name", label: "Name" },
] as const;

type ArcaneCollectionOrdering =
  (typeof ARCANE_COLLECTION_ORDERINGS)[number]["value"];

interface ArcaneCollectionFilters {
  status: ArcaneCollectionStatus | null;
  favourite: YesNo | null;
}

function rankSummary(row: ArcaneCollectionRow): string {
  if (row.ranks.length === 0) {
    return `Max rank ${row.max_rank}`;
  }

  return row.ranks
    .map(({ rank, count }) => `R${rank} x${num(count)}`)
    .join(", ");
}

function ArcaneCollectionCardInner({
  row,
  item,
  favourite,
}: ArcaneCollectionEntry) {
  const openListing = useMarketPanelStore((state) => state.openListing);
  const slug = item.market_slug;
  const collected = row.remaining === 0;

  return (
    <div className="bg-card hover:border-primary/50 flex gap-4 rounded-xl border p-4 transition-colors">
      <ArcaneImage
        imageName={item.image_name}
        rarity={row.rarity}
        size={80}
        alt={item.name}
        className={cn(
          "bg-muted/60 self-start",
          row.status === "not_owned" && "opacity-35 grayscale",
        )}
      />
      <div className="flex min-w-0 flex-1 flex-col gap-1.5">
        <div className="flex items-center justify-between gap-2">
          <span className="truncate text-sm font-semibold" title={item.name}>
            {item.name}
          </span>
          <span className="-my-1 -mr-1 flex shrink-0 items-center">
            <FavouriteStar
              uniqueName={item.unique_name}
              favourite={favourite}
            />
          </span>
        </div>
        <div className="flex items-center gap-2">
          <Progress
            value={Math.min(100, (row.copies / row.required) * 100)}
            className="h-1.5 flex-1"
            indicatorClassName={collected ? "bg-accent" : undefined}
          />
          <Hint as="span" className="min-w-12 shrink-0 text-right">
            {num(row.copies)}/{num(row.required)}
          </Hint>
        </div>
        <div className="flex items-baseline justify-between gap-2">
          <span
            className={cn(
              "truncate text-sm",
              collected ? "text-accent" : "text-muted-foreground",
            )}
          >
            {row.status === "collecting"
              ? missingCopies(row)
              : ARCANE_STATUS_LABELS[row.status]}
          </span>
          <Hint as="span" className="shrink-0">
            {rankSummary(row)}
          </Hint>
        </div>
        <div className="flex h-8 items-center justify-between gap-2">
          {row.finish_cost !== null && (
            <Hint
              as="span"
              className="flex items-center gap-1"
              title="What the missing copies cost at the current price of an unranked one"
            >
              <Amount currency="platinum" size={14}>
                {num(row.finish_cost)}
              </Amount>
              to finish
            </Hint>
          )}
          {slug && !collected && (
            <RankedPlat
              side="buy"
              value={row.sell}
              className="ml-auto basis-24 grow-0"
              onOpen={() => openListing(slug, "buy", 0)}
            />
          )}
        </div>
      </div>
    </div>
  );
}

const ArcaneCollectionCard = memo(ArcaneCollectionCardInner);

export function ArcaneCollectionView({ data }: { data: InventoryTab }) {
  const [search, setSearch] = useState("");
  const [ordering, setOrdering] = useState<ArcaneCollectionOrdering>("missing");
  const filters = useFilters<ArcaneCollectionFilters>({
    status: null,
    favourite: null,
  });
  const { status, favourite } = filters.values;

  const entries = useMemo(() => arcaneCollectionEntries(data), [data]);

  const visible = useMemo(() => {
    const kept = entries.filter(
      (entry) =>
        matchesText(entry.item.name, search) &&
        (status === null || entry.row.status === status) &&
        (favourite === null || !failsYesNo(favourite, entry.favourite)),
    );
    return ordering === "name"
      ? kept.sort((a, b) => collator.compare(a.item.name, b.item.name))
      : kept;
  }, [entries, search, status, favourite, ordering]);

  const { pageItems, pagination } = usePaged(
    visible,
    JSON.stringify({ search, ordering, ...filters.values }),
  );

  useEffect(() => {
    prefetchImages(pageItems.map((entry) => entry.item.image_name));
  }, [pageItems]);

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <SearchInput
          value={search}
          onValueChange={setSearch}
          placeholder="Search"
        />
        <OptionSelect
          label="Order by"
          value={ordering}
          options={ARCANE_COLLECTION_ORDERINGS}
          onChange={setOrdering}
          className="w-44"
        />
        <span className="text-muted-foreground ml-auto text-xs">
          {num(visible.length)} of {num(entries.length)} shown
        </span>
      </div>

      <FilterGrid activeFilters={filters.active} onClear={filters.clear}>
        <FilterSelect
          label="Collection"
          value={status}
          options={ARCANE_STATUS_OPTIONS}
          onChange={(value) => filters.set("status", value)}
        />
        <FilterSelect
          label="Favourite"
          value={favourite}
          options={FAVOURITE}
          onChange={(value) => filters.set("favourite", value)}
        />
      </FilterGrid>

      {pageItems.length === 0 ? (
        <EmptyPanel>{NO_FILTER_MATCHES}</EmptyPanel>
      ) : (
        <CardGrid wide>
          {pageItems.map((entry) => (
            <ArcaneCollectionCard key={entry.item.unique_name} {...entry} />
          ))}
        </CardGrid>
      )}

      <Pagination {...pagination} />
    </div>
  );
}
