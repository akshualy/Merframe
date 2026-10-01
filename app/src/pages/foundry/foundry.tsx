import { useCallback, useEffect, useMemo, useState } from "react";
import { FilterGrid } from "@/components/filter-grid";
import { FilterSelect } from "@/components/filter-select";
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
import { useCommand } from "@/hooks/use-command";
import { usePaged } from "@/hooks/use-paged";
import { type YesNo, yesNoOptions } from "@/lib/filters";
import { num } from "@/lib/format";
import {
  CATEGORIES,
  type Category,
  type Filters,
  filtersFor,
  foundryMatches,
  scoped,
} from "@/lib/foundry-filters";
import { usePageQuote } from "@/lib/quotes";
import { usePreferencesStore } from "@/stores/preferences-store";
import { FoundryCard } from "./card";
import { FoundryQueue } from "./queue";
import { FoundryTreeDialog } from "./tree-dialog";

export function FoundryPage() {
  const quote = usePageQuote("foundry");
  const { data, error, loading } = useCommand("foundry");
  const { foundryCategory: category, setFoundryCategory } =
    usePreferencesStore();
  const [query, setQuery] = useState("");
  const [filters, setFilters] = useState<Filters>({});
  const [selectedName, setSelectedName] = useState<string | null>(null);

  const handleCategoryChange = (next: Category) => {
    setFoundryCategory(next);
    setFilters((current) => scoped(current, next));
  };

  const visible = useMemo(
    () =>
      (data?.items ?? [])
        .filter((item) => category === "all" || item.kind === category)
        .filter((item) => foundryMatches(item, query, filters)),
    [data?.items, category, query, filters],
  );

  const { pageItems, pagination } = usePaged(
    visible,
    JSON.stringify([category, query, filters]),
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

  return (
    <Page title="Foundry" description={<Quoted quote={quote} />}>
      {error && <ErrorNote message={error} />}

      {data && data.pending.length > 0 && (
        <FoundryQueue builds={data.pending} />
      )}

      <Section
        action={
          <div className="flex items-center gap-3">
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
            onValueChange={(value) => handleCategoryChange(value as Category)}
          >
            <TabsList>
              {CATEGORIES.map(([key, label]) => (
                <TabsTrigger key={key} value={key}>
                  {label}
                </TabsTrigger>
              ))}
            </TabsList>
          </Tabs>

          <FilterGrid
            activeFilters={activeFilters}
            onClear={() => setFilters({})}
          >
            {filtersFor(category).map((group) => (
              <FilterSelect
                key={group.key}
                label={group.label}
                value={filters[group.key] ?? null}
                options={yesNoOptions(group.yes, group.no)}
                onChange={(mode) => {
                  const next = { ...filters };
                  if (mode === null) {
                    delete next[group.key];
                  } else {
                    next[group.key] = mode as YesNo;
                  }
                  setFilters(next);
                }}
              />
            ))}
          </FilterGrid>

          {pageItems.length === 0 ? (
            <EmptyPanel>Nothing matches these filters.</EmptyPanel>
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
