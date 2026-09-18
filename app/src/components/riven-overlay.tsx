import { Check, ExternalLink, Minus } from "lucide-react";
import { useCallback, useEffect } from "react";
import { PolarityIcon } from "@/components/game-icon";
import {
  desiredAttributes,
  GoodRollAttribution,
  GoodRollBlock,
} from "@/components/good-roll";
import { ItemImage, prefetchImages } from "@/components/item-image";
import { RivenListings } from "@/components/riven-listings";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import { api, reportError } from "@/lib/bridge";
import { displayNameFromPath, percent } from "@/lib/format";
import {
  attributeValue,
  gradeTone,
  rivenAuctionUrl,
  rollQuality,
} from "@/lib/rivens";
import { cn } from "@/lib/utils";
import type { AttributeGrade, RivenRow } from "@/types";

function rivenName(riven: RivenRow): string {
  const parts = [riven.weapon, riven.name].filter(
    (part): part is string => part !== null,
  );
  return parts.length > 0
    ? parts.join(" ")
    : displayNameFromPath(riven.item_type);
}

function attributeLabel(attribute: AttributeGrade): string {
  return attribute.name ?? attribute.tag;
}

function Attribute({
  attribute,
  desired,
}: {
  attribute: AttributeGrade;
  desired: boolean;
}) {
  return (
    <div className="grid grid-cols-subgrid col-span-4 text-sm items-center">
      <span
        className={cn(
          "w-14 shrink-0 text-right tabular-nums",
          attribute.curse && "text-warning",
        )}
      >
        {attributeValue(attribute) ?? percent(attribute.percentile, 0)}
      </span>
      <span className="truncate">{attributeLabel(attribute)}</span>
      <span className={cn("w-6 shrink-0", gradeTone(attribute.grade))}>
        {attribute.grade}
      </span>
      {desired ? (
        <Check className="text-accent size-3.5 shrink-0" aria-label="Desired" />
      ) : (
        <Minus className="text-muted-foreground size-3.5 shrink-0" />
      )}
    </div>
  );
}

function RivenCard({ riven, compact }: { riven: RivenRow; compact: boolean }) {
  const quality = rollQuality(riven.grade);
  const desired = riven.good_roll
    ? desiredAttributes(riven.good_roll)
    : new Set<string>();
  const slug = riven.weapon_slug;

  const handleCompare = useCallback(async () => {
    if (!slug) {
      return;
    }
    try {
      await api.openUrl(rivenAuctionUrl(slug));
    } catch (error) {
      reportError(error);
    }
  }, [slug]);

  return (
    <div className="flex w-80 max-w-full flex-col rounded-lg border gap-3 px-2 py-1.5">
      <div className="flex items-center gap-2">
        <ItemImage
          imageName={riven.image_name}
          size={compact ? 28 : 32}
          alt={riven.weapon ?? "Riven"}
        />
        <div className="flex min-w-0 flex-col">
          <span className="flex items-center gap-1 text-sm font-medium">
            {rivenName(riven)}
            {riven.polarity && (
              <PolarityIcon polarity={riven.polarity} size={16} />
            )}
          </span>
          {riven.disposition !== null && (
            <Hint as="span">
              Dispo x{riven.disposition.toFixed(2)}
              {riven.disposition_weapon
                ? ` on ${riven.disposition_weapon}`
                : ""}
            </Hint>
          )}
        </div>
        <div className="ml-auto flex items-center gap-2">
          <Badge variant={quality.variant}>{quality.label}</Badge>
          <span className="text-xs tabular-nums">{percent(riven.grade)}</span>
        </div>
      </div>
      <div className="grid grid-cols-[auto_minmax(0,1fr)_auto_auto] gap-1.5">
        {riven.attributes.map((attribute) => (
          <Attribute
            key={attribute.tag}
            attribute={attribute}
            desired={desired.has(attributeLabel(attribute))}
          />
        ))}
      </div>
      {riven.good_roll && (
        <GoodRollBlock view={riven.good_roll} className="border-t pt-1.5" />
      )}
      {riven.weapon_slug && (
        <RivenListings
          weaponSlug={riven.weapon_slug}
          attributes={riven.attributes}
          limit={compact ? 3 : 5}
          className="border-t pt-1.5"
        />
      )}
      {riven.weapon_slug && !compact && (
        <Button
          variant="outline"
          size="sm"
          className="self-start"
          onClick={handleCompare}
        >
          <ExternalLink className="size-3.5" />
          Compare on warframe.market
        </Button>
      )}
    </div>
  );
}

export function RivenRerollOverlay({
  before,
  after,
  attribution = null,
  compact = false,
}: {
  before: RivenRow;
  after: RivenRow | null;
  attribution?: string | null;
  compact?: boolean;
}) {
  useEffect(() => {
    prefetchImages([before.image_name]);
  }, [before.image_name]);

  if (!after) {
    return (
      <div className="flex flex-col gap-1">
        <RivenCard riven={before} compact={compact} />
        <GoodRollAttribution attribution={attribution} />
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex flex-wrap gap-2">
        <RivenCard riven={before} compact={compact} />
        <RivenCard riven={after} compact={compact} />
      </div>
      <GoodRollAttribution attribution={attribution} />
    </div>
  );
}

export function RivenOverlay({
  rivens,
  attribution = null,
  compact = false,
}: {
  rivens: RivenRow[];
  attribution?: string | null;
  compact?: boolean;
}) {
  useEffect(() => {
    prefetchImages(rivens.map((riven) => riven.image_name));
  }, [rivens]);

  return (
    <div className={cn("flex flex-col", compact ? "gap-1" : "gap-2")}>
      {rivens.map((riven) => (
        <RivenCard key={riven.item_id} riven={riven} compact={compact} />
      ))}
      <GoodRollAttribution attribution={attribution} />
    </div>
  );
}
