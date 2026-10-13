import { useCallback, useEffect, useMemo, useState } from "react";
import { ClearFilters, FilterGrid } from "@/components/filter-grid";
import { FilterSegment } from "@/components/filter-segment";
import { prefetchImages } from "@/components/item-image";
import {
  CardGrid,
  EmptyPanel,
  ErrorNote,
  Page,
  Quoted,
  Section,
  TableSkeleton,
} from "@/components/page";
import { Pagination } from "@/components/pagination";
import { SearchInput } from "@/components/search-input";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useAsyncData } from "@/hooks/use-async-data";
import { usePaged } from "@/hooks/use-paged";
import { api } from "@/lib/bridge";
import { num } from "@/lib/format";
import {
  CATEGORIES,
  type Category,
  choice,
  FILTERS,
  type Filters,
  facets,
  foundryMatches,
  inCategory,
  offered,
  prune,
  withFilter,
} from "@/lib/foundry-filters";
import { usePageQuote } from "@/lib/quotes";
import { usePreferencesStore } from "@/stores/preferences-store";
import { FoundryCard } from "./card";
import { FoundryQueue } from "./queue";
import { FoundryTreeDialog } from "./tree-dialog";

export function FoundryPage() {
  const quote = usePageQuote("foundry");
  const { data, error, loading } = useAsyncData("foundry_tab", api.foundryTab, [
    "inventoryUpdated",
    "pricesUpdated",
    "favouriteUpdated",
  ]);
  const category = usePreferencesStore((state) => state.foundryCategory);
  const setCategory = usePreferencesStore((state) => state.setFoundryCategory);
  const [query, setQuery] = useState("");
  const [stored, setFilters] = useState<Filters>({});
  const [selectedName, setSelectedName] = useState<string | null>(null);

  const items = useMemo(
    () => inCategory(data?.items ?? [], category),
    [data?.items, category],
  );
  const filters = useMemo(() => prune(items, stored), [items, stored]);
  const base = useMemo(() => facets(items, {}), [items]);
  const live = useMemo(() => facets(items, filters), [items, filters]);
  const visible = useMemo(
    () => items.filter((item) => foundryMatches(item, query, filters)),
    [items, query, filters],
  );

  const { pageItems, pagination } = usePaged(
    visible,
    JSON.stringify([category, query]),
  );

  const handleCloseTree = useCallback(() => setSelectedName(null), []);
  const selected = useMemo(
    () => data?.items.find((item) => item.unique_name === selectedName) ?? null,
    [data?.items, selectedName],
  );

  useEffect(() => {
    prefetchImages([
      ...pageItems.map((item) => item.image_name),
      ...pageItems.flatMap((item) =>
        item.components.map((component) => component.image_name),
      ),
    ]);
  }, [pageItems]);

  if (loading && !data) {
    return (
      <Page title="Foundry" description={<Quoted quote={quote} />}>
        <TableSkeleton />
      </Page>
    );
  }

  const activeFilters = Object.keys(filters).length;
  const groups = FILTERS.filter((group) => offered(base[group.key]));

  return (
    <Page title="Foundry" description={<Quoted quote={quote} />}>
      {error && <ErrorNote message={error} />}

      {data && data.pending.length > 0 && (
        <FoundryQueue builds={data.pending} />
      )}

      <Section
        action={
          <div className="flex items-center gap-3">
            {activeFilters > 0 && (
              <ClearFilters
                count={activeFilters}
                onClear={() => setFilters({})}
              />
            )}
            <span className="text-muted-foreground text-sm">
              {num(visible.length)} of {num(data?.items.length)} items
            </span>
            <SearchInput
              value={query}
              onValueChange={setQuery}
              placeholder="Search"
            />
          </div>
        }
      >
        <div className="flex flex-col gap-3">
          <Tabs
            value={category}
            onValueChange={(value) => setCategory(value as Category)}
          >
            <TabsList>
              {CATEGORIES.map(([key, label]) => (
                <TabsTrigger key={key} value={key}>
                  {label}
                </TabsTrigger>
              ))}
            </TabsList>
          </Tabs>

          <FilterGrid>
            {groups.map((group) => {
              const { value, fixed } = choice(
                live[group.key],
                filters[group.key],
              );
              return (
                <FilterSegment
                  key={group.key}
                  label={group.label}
                  yes={group.yes}
                  no={group.no}
                  facet={live[group.key]}
                  value={value}
                  fixed={fixed}
                  onChange={(mode) =>
                    setFilters(withFilter(items, filters, group.key, mode))
                  }
                />
              );
            })}
          </FilterGrid>

          {pageItems.length === 0 ? (
            <EmptyPanel>Nothing matches.</EmptyPanel>
          ) : (
            <CardGrid>
              {pageItems.map((item) => (
                <FoundryCard
                  key={item.unique_name}
                  item={item}
                  onOpen={setSelectedName}
                />
              ))}
            </CardGrid>
          )}

          <Pagination {...pagination} />
        </div>
      </Section>

      <FoundryTreeDialog item={selected} onClose={handleCloseTree} />
    </Page>
  );
}
