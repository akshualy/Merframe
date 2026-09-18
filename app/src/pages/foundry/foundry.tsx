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
import { SearchInput } from "@/components/search-input";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useCommand } from "@/hooks/use-command";
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
  const [showAll, setShowAll] = useState(false);
  const [selectedName, setSelectedName] = useState<string | null>(null);

  const handleCategoryChange = (next: Category) => {
    setFoundryCategory(next);
    setFilters((current) => scoped(current, next));
    setShowAll(false);
  };

  const visible = useMemo(
    () =>
      (data?.items ?? [])
        .filter((item) => category === "all" || item.kind === category)
        .filter((item) => foundryMatches(item, query, filters)),
    [data?.items, category, query, filters],
  );

  const cappedItems = useMemo(
    () => (showAll ? visible : visible.slice(0, 100)),
    [visible, showAll],
  );

  const handleCloseTree = useCallback(() => setSelectedName(null), []);
  const selected = useMemo(
    () => data?.items.find((item) => item.unique_name === selectedName) ?? null,
    [data?.items, selectedName],
  );

  useEffect(() => {
    prefetchImages([
      ...cappedItems.map((item) => item.image_name),
      ...cappedItems.flatMap((item) =>
        item.components.map((component) => component.image_name),
      ),
    ]);
  }, [cappedItems]);

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
              onChange={(e) => setQuery(e.target.value)}
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
                  setShowAll(false);
                }}
              />
            ))}
          </FilterGrid>

          {cappedItems.length === 0 ? (
            <EmptyPanel>Nothing matches these filters.</EmptyPanel>
          ) : (
            <CardGrid>
              {cappedItems.map((item) => (
                <FoundryCard
                  key={item.unique_name}
                  item={item}
                  onOpen={setSelectedName}
                />
              ))}
            </CardGrid>
          )}

          {visible.length > cappedItems.length && (
            <p className="text-muted-foreground text-center text-xs">
              Only the first {num(cappedItems.length)} of {num(visible.length)}{" "}
              are shown.{" "}
              <button
                type="button"
                className="text-primary cursor-pointer underline-offset-2 hover:underline"
                onClick={() => setShowAll(true)}
              >
                Show All
              </button>
            </p>
          )}
        </div>
      </Section>

      <FoundryTreeDialog item={selected} onClose={handleCloseTree} />
    </Page>
  );
}
