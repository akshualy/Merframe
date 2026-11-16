import type { ColumnDef } from "@tanstack/react-table";
import { type ReactNode, useMemo } from "react";
import { Amount } from "@/components/currency";
import { DataTable } from "@/components/data-table";
import { FilterGrid } from "@/components/filter-grid";
import { FilterSelect } from "@/components/filter-select";
import { ItemImage } from "@/components/item-image";
import { Section } from "@/components/page";
import { Dash } from "@/components/ui/dash";
import { Hint } from "@/components/ui/hint";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useFilters } from "@/hooks/use-filters";
import {
  type FilterOption,
  failsYesNo,
  type YesNo,
  yesNoOptions,
} from "@/lib/filters";
import { num, plat } from "@/lib/format";
import type { SyndicateOfferRow, SyndicateOverview } from "@/types";

const CRAFTED_MARK = "Priced as crafted";

interface SyndicateOfferEntry extends SyndicateOfferRow {
  tag: string;
  syndicate: string;
}

const STANDING_BANDS = [
  { value: "low", from: 0, to: 5_000 },
  { value: "middle", from: 5_001, to: 25_000 },
  { value: "high", from: 25_001, to: 100_000 },
  { value: "top", from: 100_001, to: Infinity },
] as const;

type SyndicateStandingBand = (typeof STANDING_BANDS)[number]["value"];

const STANDING_OPTIONS: FilterOption<SyndicateStandingBand>[] =
  STANDING_BANDS.map(({ value, from, to }) => ({
    value,
    label:
      to === Infinity
        ? `Over ${num(from - 1)}`
        : from === 0
          ? `Up to ${num(to)}`
          : `${num(from)} to ${num(to)}`,
  }));

const LEAST_TRADES = ["10", "50", "100", "500"] as const;

type SyndicateLeastTrades = (typeof LEAST_TRADES)[number];

const LEAST_TRADES_OPTIONS: FilterOption<SyndicateLeastTrades>[] =
  LEAST_TRADES.map((value) => ({
    value,
    label: `${num(Number(value))} or more`,
  }));

type SyndicateOfferFilters = {
  syndicate: string | null;
  rank: YesNo | null;
  owned: YesNo | null;
  standing: SyndicateStandingBand | null;
  trades: SyndicateLeastTrades | null;
};

const DEFAULT_OFFER_FILTERS: SyndicateOfferFilters = {
  syndicate: null,
  rank: null,
  owned: null,
  standing: null,
  trades: "10",
};

const RANK_OPTIONS = yesNoOptions("Your ranks", "Above your rank");
const OWNED_OPTIONS = yesNoOptions("Owned", "Not owned");

function passesOfferFilters(
  offer: SyndicateOfferEntry,
  filters: SyndicateOfferFilters,
): boolean {
  const band = STANDING_BANDS.find(({ value }) => value === filters.standing);

  return (
    (filters.syndicate === null || filters.syndicate === offer.tag) &&
    (filters.rank === null || !failsYesNo(filters.rank, offer.available)) &&
    (filters.owned === null || !failsYesNo(filters.owned, offer.owned)) &&
    (band === undefined ||
      (offer.standing_cost >= band.from && offer.standing_cost <= band.to)) &&
    (filters.trades === null ||
      (offer.weekly_trades ?? 0) >= Number(filters.trades))
  );
}

function sortedLastWhenUnknown(
  id: string,
  header: string,
  value: (offer: SyndicateOfferEntry) => number | null,
  cell: (value: number) => ReactNode,
): ColumnDef<SyndicateOfferEntry> {
  return {
    id,
    header,
    meta: { numeric: true },
    accessorFn: (offer) => value(offer) ?? undefined,
    sortUndefined: "last",
    cell: ({ row }) => {
      const known = value(row.original);
      return known === null ? <Dash /> : cell(known);
    },
  };
}

const OFFER_COLUMNS: ColumnDef<SyndicateOfferEntry>[] = [
  {
    accessorKey: "name",
    header: "Offer",
    enableHiding: false,
    cell: ({ row }) => (
      <>
        <ItemImage imageName={row.original.image_name} size={24} />
        <span className="font-medium">{row.original.name}</span>
        {row.original.crafted && <Hint as="span">{CRAFTED_MARK}</Hint>}
      </>
    ),
  },
  { accessorKey: "syndicate", header: "Syndicate" },
  {
    accessorKey: "required_rank",
    header: "Rank",
    meta: { numeric: true },
    cell: ({ row }) =>
      row.original.available ? (
        row.original.required_rank
      ) : (
        <Tooltip>
          <TooltipTrigger asChild>
            <span className="text-warning">{row.original.required_rank}</span>
          </TooltipTrigger>
          <TooltipContent>Above your rank</TooltipContent>
        </Tooltip>
      ),
  },
  {
    accessorKey: "standing_cost",
    header: "Standing",
    meta: { numeric: true },
    cell: ({ row }) => num(row.original.standing_cost),
  },
  sortedLastWhenUnknown(
    "platinum",
    "Platinum",
    (offer) => offer.platinum,
    (value) => <Amount currency="platinum">{num(value)}</Amount>,
  ),
  sortedLastWhenUnknown(
    "platinum_per_thousand",
    "Per 1,000 standing",
    (offer) => offer.platinum_per_thousand,
    (value) => <Amount currency="platinum">{plat(value)}</Amount>,
  ),
  sortedLastWhenUnknown(
    "weekly_trades",
    "Trades (7 days)",
    (offer) => offer.weekly_trades,
    num,
  ),
  {
    accessorKey: "owned",
    header: "Owned",
    cell: ({ row }) => (row.original.owned ? "Yes" : "No"),
  },
];

export function useSyndicateOffers(syndicates: SyndicateOverview[]) {
  const filters = useFilters<SyndicateOfferFilters>(DEFAULT_OFFER_FILTERS);
  const offers = useMemo(
    () =>
      syndicates.flatMap((syndicate) =>
        syndicate.offers.map((offer) => ({
          ...offer,
          tag: syndicate.tag,
          syndicate: syndicate.name,
        })),
      ),
    [syndicates],
  );

  const chosen = {
    ...filters.values,
    syndicate:
      syndicates.find(({ tag }) => tag === filters.values.syndicate)?.tag ??
      null,
  };

  return {
    filters,
    chosen,
    offers: offers.filter((offer) => passesOfferFilters(offer, chosen)),
  };
}

export function SyndicateOffers({
  syndicates,
  selection: { filters, chosen, offers },
  onSearchChange,
}: {
  syndicates: SyndicateOverview[];
  selection: ReturnType<typeof useSyndicateOffers>;
  onSearchChange: (search: string) => void;
}) {
  return (
    <Section title="Offers">
      <DataTable
        tableId="syndicate-offers"
        columns={OFFER_COLUMNS}
        data={offers}
        resetKey={JSON.stringify(chosen)}
        searchPlaceholder="Search offers"
        searchValue={(offer) => offer.search_text}
        onSearchChange={onSearchChange}
        initialSorting={[{ id: "platinum_per_thousand", desc: true }]}
        rowKey={(offer) =>
          `${offer.tag}/${offer.name}/${offer.required_rank}/${offer.standing_cost}`
        }
        emptyMessage="No offer matches."
        filters={
          <FilterGrid activeFilters={filters.active} onClear={filters.clear}>
            <FilterSelect
              label="Syndicate"
              value={chosen.syndicate}
              options={syndicates.map((syndicate) => ({
                value: syndicate.tag,
                label: syndicate.name,
              }))}
              onChange={(value) => filters.set("syndicate", value)}
            />
            <FilterSelect
              label="Rank"
              value={filters.values.rank}
              options={RANK_OPTIONS}
              onChange={(value) => filters.set("rank", value)}
            />
            <FilterSelect
              label="Standing cost"
              value={filters.values.standing}
              options={STANDING_OPTIONS}
              onChange={(value) => filters.set("standing", value)}
            />
            <FilterSelect
              label="Owned"
              value={filters.values.owned}
              options={OWNED_OPTIONS}
              onChange={(value) => filters.set("owned", value)}
            />
            <FilterSelect
              label="Trades in 7 days"
              value={filters.values.trades}
              options={LEAST_TRADES_OPTIONS}
              onChange={(value) => filters.set("trades", value)}
            />
          </FilterGrid>
        }
      />
    </Section>
  );
}
