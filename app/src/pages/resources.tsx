import { useEffect, useMemo, useState } from "react";
import { FilterGrid } from "@/components/filter-grid";
import { FilterSingleSelect } from "@/components/filter-select";
import { ItemImage, prefetchImages } from "@/components/item-image";
import {
  CardGrid,
  EmptyNote,
  ErrorNote,
  Page,
  Quoted,
  Section,
  Stat,
  TableSkeleton,
} from "@/components/page";
import { SearchInput } from "@/components/search-input";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { useCommand } from "@/hooks/use-command";
import { num } from "@/lib/format";
import { usePageQuote } from "@/lib/quotes";
import type { ResourceRow } from "@/types";

type Scope = "all" | "short";

const SCOPES: readonly { value: Scope; label: string }[] = [
  { value: "all", label: "All resources" },
  { value: "short", label: "Only shortfalls" },
];

function ResourceCard({ row }: { row: ResourceRow }) {
  const ratio = Math.min(1, row.owned / Math.max(row.required, 1));
  const short = row.deficit > 0;
  return (
    <div className="bg-card flex items-center gap-3 rounded-xl border p-3">
      <ItemImage imageName={row.image_name} size={48} alt={row.name} />
      <div className="flex min-w-0 flex-1 flex-col gap-1.5">
        <div className="flex items-center justify-between gap-2">
          <span className="truncate text-sm font-medium">{row.name}</span>
          {short ? (
            <Badge variant="warning">short {num(row.deficit)}</Badge>
          ) : (
            <Badge variant="accent">covered</Badge>
          )}
        </div>
        <Progress
          value={Math.round(ratio * 100)}
          className="h-1.5"
          indicatorClassName={short ? "bg-warning" : undefined}
        />
        <div className="text-muted-foreground flex justify-between text-xs tabular-nums">
          <span>
            <span className="text-foreground">{num(row.owned)}</span> owned
          </span>
          <span>{num(row.required)} needed</span>
        </div>
      </div>
    </div>
  );
}

export function ResourcesPage() {
  const quote = usePageQuote("resources");
  const { data, error, loading } = useCommand("resources");
  const [query, setQuery] = useState("");
  const [scope, setScope] = useState<Scope>("all");

  const rows = useMemo(() => {
    const search = query.trim().toLowerCase();
    return (data?.resources ?? [])
      .filter((row) => (scope === "short" ? row.deficit > 0 : true))
      .filter((row) =>
        search ? row.name.toLowerCase().includes(search) : true,
      )
      .sort((left, right) => {
        if (left.deficit !== right.deficit) {
          return right.deficit - left.deficit;
        }
        return left.name.localeCompare(right.name);
      });
  }, [data?.resources, query, scope]);

  const recipes = useMemo(
    () =>
      [...(data?.recipes ?? [])].sort(
        (left, right) => right.count - left.count,
      ),
    [data?.recipes],
  );

  useEffect(() => {
    prefetchImages(rows.map((row) => row.image_name));
  }, [rows]);

  if (loading && !data) {
    return (
      <Page title="Resources" description={<Quoted quote={quote} />}>
        <TableSkeleton />
      </Page>
    );
  }

  const short = data?.resources.filter((row) => row.deficit > 0).length ?? 0;

  return (
    <Page title="Resources" description={<Quoted quote={quote} />}>
      {error && <ErrorNote message={error} />}

      <div className="grid gap-4 sm:grid-cols-3">
        <Stat label="Tracked resources" value={num(data?.resources.length)} />
        <Stat label="Short of target" value={num(short)} />
        <Stat label="Blueprints in demand" value={num(data?.recipes.length)} />
      </div>

      <Section
        title="Stockpile"
        action={
          <SearchInput
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search"
          />
        }
      >
        <div className="flex flex-col gap-3">
          <FilterGrid>
            <FilterSingleSelect
              label="Show"
              value={scope}
              options={SCOPES}
              onChange={setScope}
            />
          </FilterGrid>

          {rows.length === 0 ? (
            <EmptyNote>Nothing matches these filters.</EmptyNote>
          ) : (
            <CardGrid>
              {rows.map((row) => (
                <ResourceCard key={row.unique_name} row={row} />
              ))}
            </CardGrid>
          )}
        </div>
      </Section>

      <Section title="Where the Demand Comes From">
        {recipes.length === 0 ? (
          <EmptyNote>
            No blueprints with resource costs are in the inventory.
          </EmptyNote>
        ) : (
          <div className="flex flex-wrap gap-2">
            {recipes.map((recipe) => (
              <Badge key={recipe.recipe} variant="secondary">
                {recipe.name}
                {recipe.count > 1 && (
                  <span className="opacity-70">x{num(recipe.count)}</span>
                )}
              </Badge>
            ))}
          </div>
        )}
      </Section>
    </Page>
  );
}
