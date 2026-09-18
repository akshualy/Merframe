import { useEffect, useState } from "react";
import { GameIcon } from "@/components/game-icon";
import { Hint } from "@/components/ui/hint";
import { api, logError } from "@/lib/bridge";
import { num, percent } from "@/lib/format";
import { attributeAmount } from "@/lib/rivens";
import { cn } from "@/lib/utils";
import type {
  AttributeGrade,
  ComparableListing,
  ComparedStat,
  RivenComparables,
} from "@/types";

function comparedStats(attributes: AttributeGrade[]): ComparedStat[] {
  return attributes.flatMap((attribute) =>
    attribute.slug === null
      ? []
      : [{ slug: attribute.slug, positive: !attribute.curse }],
  );
}

function updatedAgo(unixSeconds: number): string {
  const minutes = Math.max(
    0,
    Math.round((Date.now() / 1000 - unixSeconds) / 60),
  );
  if (minutes < 1) {
    return "just now";
  }
  if (minutes < 60) {
    return `${minutes} min ago`;
  }
  return `${Math.floor(minutes / 60)} h ago`;
}

function Listing({ listing }: { listing: ComparableListing }) {
  const exact = listing.similarity >= 0.999;
  return (
    <div className="flex w-24 flex-col gap-0.5 rounded-md border px-1 py-1 text-xs">
      <div className="flex items-center justify-between gap-1">
        <span className="flex items-center gap-0.5 tabular-nums font-medium">
          {num(listing.price)}
          <GameIcon
            name="platinum"
            size={12}
            alt="Platinum"
            className="-translate-y-0.5"
          />
        </span>
        <span
          className={cn(
            "tabular-nums",
            exact ? "text-accent" : "text-muted-foreground",
          )}
        >
          {percent(listing.similarity, 0)}
        </span>
      </div>
      {listing.attributes.map((attribute) => (
        <span
          key={attribute.name}
          title={attribute.name}
          className={cn(
            "flex gap-1 whitespace-nowrap",
            attribute.shared ? "text-foreground" : "text-muted-foreground",
          )}
        >
          <span
            className={cn(
              "tabular-nums",
              !attribute.positive && "text-warning",
            )}
          >
            {attributeAmount(attribute.unit, attribute.value)}
          </span>
          {attribute.abbr ?? attribute.name}
        </span>
      ))}
    </div>
  );
}

export function RivenListings({
  weaponSlug,
  attributes,
  limit,
  className,
}: {
  weaponSlug: string;
  attributes: AttributeGrade[];
  limit: number;
  className?: string;
}) {
  const [found, setFound] = useState<RivenComparables | null>(null);
  const [missing, setMissing] = useState(false);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      try {
        const next = await api.rivenComparables(
          weaponSlug,
          comparedStats(attributes),
        );
        if (cancelled) {
          return;
        }
        setMissing(next === null);
        if (next) {
          setFound(next);
        }
      } catch (error) {
        logError("Loading the riven listings", error);
      }
    }
    load();
    return () => {
      cancelled = true;
    };
  }, [weaponSlug, attributes]);

  if (missing) {
    return (
      <Hint className={className}>
        No warframe.market listings known for this weapon yet
      </Hint>
    );
  }
  if (!found) {
    return null;
  }
  const shownListings = found.listings.slice(0, limit);
  return (
    <div className={cn("flex flex-col gap-1.5", className)}>
      <div className="flex items-baseline justify-between gap-2">
        <span className="text-primary text-sm">Market listings</span>
        <Hint as="span">
          {found.lowest === null ? (
            "none listed"
          ) : (
            <span className="inline-flex items-center gap-0.5">
              {num(found.listed)} listed
            </span>
          )}
          , {updatedAgo(found.updated_at)}
        </Hint>
      </div>
      {shownListings.length === 0 ? (
        <Hint>No listing shares at least half of these stats</Hint>
      ) : (
        <div className="flex flex-wrap gap-1.5">
          {shownListings.map((listing) => (
            <Listing key={listing.id} listing={listing} />
          ))}
        </div>
      )}
    </div>
  );
}
