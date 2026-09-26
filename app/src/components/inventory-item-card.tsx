import { ExternalLink } from "lucide-react";
import { memo } from "react";
import { Link } from "react-router";
import { EquippedDialog } from "@/components/equipped-dialog";
import { FavouriteStar } from "@/components/favourite-star";
import { GameIcon, relicTierIcon } from "@/components/game-icon";
import { ArcaneImage, ItemImage } from "@/components/item-image";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import { Progress } from "@/components/ui/progress";
import { api, reportError } from "@/lib/bridge";
import { marketListingPath, marketUrl, num } from "@/lib/format";
import type { InventoryTabKey } from "@/lib/inventory-filters";
import { equippedLabel, type Row } from "@/lib/inventory-rows";
import { refinementTone } from "@/lib/relics";
import { cn } from "@/lib/utils";
import type { SetRow } from "@/types";

function Ducats({ ducats }: { ducats: number | null }) {
  if (!ducats) {
    return null;
  }
  return (
    <span className="text-accent flex items-center justify-center gap-0.5 text-xs font-medium tabular-nums">
      {num(ducats)}
      <GameIcon name="ducats" size={16} alt="Ducats" />
    </span>
  );
}

function RankedPlat({
  label,
  value,
  floor,
  tone,
  to,
}: {
  label: string;
  value: number | null;
  floor?: boolean;
  tone?: string;
  to?: string;
}) {
  const body = (
    <>
      <Hint as="span">{label}</Hint>
      {value === null ? (
        <span className="text-muted-foreground">-</span>
      ) : (
        <span
          className={cn(
            "flex items-center gap-0.5 text-xs font-bold tabular-nums",
            tone ?? "text-primary",
          )}
        >
          {num(value)}
          {floor ? "+" : ""}
          <GameIcon name="platinum" size={14} alt="Platinum" />
        </span>
      )}
    </>
  );
  if (to) {
    return (
      <Button variant="outline" size="sm" className="h-7 gap-1 px-2" asChild>
        <Link
          to={to}
          title={`${label === "WTB" ? "Buy" : "Sell"} on warframe.market`}
        >
          {body}
        </Link>
      </Button>
    );
  }
  return <span className="flex items-baseline gap-1 text-xs">{body}</span>;
}

function SetParts({ set }: { set: SetRow }) {
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex flex-wrap items-center gap-1">
        {set.components.map((part) => (
          <span
            key={part.unique_name}
            title={`${part.name}: ${part.owned}/${part.required}`}
            className={cn(
              "rounded-md",
              part.enough ? "ring-primary ring-2" : "opacity-35 grayscale",
            )}
          >
            <ItemImage imageName={part.image_name} size={26} />
          </span>
        ))}
      </div>
      <div className="flex items-center gap-2">
        <Progress
          value={Math.round(
            (set.owned_parts / Math.max(set.total_parts, 1)) * 100,
          )}
          className="h-1.5 flex-1"
          indicatorClassName={set.complete ? "bg-accent" : undefined}
        />
        <Hint as="span" className="tabular-nums">
          {set.owned_parts}/{set.total_parts}
        </Hint>
      </div>
    </div>
  );
}

function ItemCardInner({ row, tab }: { row: Row; tab: InventoryTabKey }) {
  const isSet = tab === "sets" && row.set;
  const equipped = equippedLabel(row);
  return (
    <div className="bg-card hover:border-primary/50 flex gap-3 rounded-xl border p-3 transition-colors">
      <div className="flex shrink-0 flex-col gap-1 self-start">
        <ArcaneImage
          imageName={row.imageName}
          rarity={tab === "arcanes" ? row.rarity : null}
          size={isSet ? 88 : 72}
          alt={row.name}
          className={tab === "mods" || tab === "arcanes" ? "bg-muted/60" : ""}
        />
        <Ducats ducats={row.ducats} />
      </div>
      <div className="flex min-w-0 flex-1 flex-col gap-1.5">
        <div className="flex items-start justify-between gap-2">
          <span className="flex min-w-0 items-start gap-1.5">
            <span className="line-clamp-2 text-sm leading-tight font-semibold">
              {row.name}
            </span>
            {isSet && row.mastered && (
              <GameIcon name="mastered" size={16} alt="Mastered" />
            )}
          </span>
          <span className="-mt-1 -mr-1 flex shrink-0 items-center">
            <FavouriteStar
              uniqueName={row.uniqueName}
              favourite={row.favourite}
              size="small"
            />
            {row.marketSlug && (
              <Button
                variant="ghost"
                size="icon"
                className="size-6 shrink-0"
                title="Open on warframe.market"
                onClick={async () => {
                  try {
                    await api.openUrl(marketUrl(row.marketSlug));
                  } catch (error) {
                    reportError(error);
                  }
                }}
              >
                <ExternalLink className="size-3.5" />
              </Button>
            )}
          </span>
        </div>
        {row.subtitle && (
          <span className="text-muted-foreground -mt-1 truncate text-xs">
            {row.subtitle}
          </span>
        )}
        {equipped && <EquippedDialog row={row} label={equipped} />}
        {row.refinement && (
          <span className="-mt-1 flex items-center gap-1.5 text-xs">
            {row.tier && (
              <GameIcon
                name={relicTierIcon(row.tier)}
                size={14}
                alt={row.tier}
              />
            )}
            <span
              className={cn("font-semibold", refinementTone(row.refinement))}
            >
              {row.refinement}
            </span>
          </span>
        )}
        {isSet && row.set && <SetParts set={row.set} />}
        <div className="mt-auto flex flex-wrap items-center gap-1.5">
          {(!isSet || row.count > 0) && (
            <Badge variant="secondary" className="tabular-nums">
              x{num(row.count)}
            </Badge>
          )}
          {row.rank !== null && <Badge variant="muted">Rank {row.rank}</Badge>}
          {row.vault !== null &&
            (row.vault === "vaulted" ? (
              <Badge variant="vaulted">Vaulted</Badge>
            ) : (
              <Badge variant="muted">Unvaulted</Badge>
            ))}
          {isSet &&
            (row.setComplete ? (
              <Badge variant="accent">Complete</Badge>
            ) : (
              <Badge variant="muted">Partial</Badge>
            ))}
          {(tab === "parts" || tab === "sets") && row.itemOwned && (
            <Badge variant="outline">Crafted</Badge>
          )}
          {row.orderPlaced && <Badge variant="outline">Order placed</Badge>}
        </div>
        <div className="flex items-center justify-end">
          {tab === "arcanes" ? (
            <span className="ml-auto flex items-baseline gap-2">
              <RankedPlat
                label="R0"
                value={row.plat}
                floor={row.platIsFloor}
                to={marketListingPath(row.marketSlug, "sell", 0)}
              />
              {(row.maxRank ?? 0) > 0 && (
                <RankedPlat
                  label={`R${row.maxRank}`}
                  value={row.platMaxRank}
                  to={marketListingPath(row.marketSlug, "sell", row.maxRank)}
                />
              )}
            </span>
          ) : (
            <span className="ml-auto flex items-baseline gap-2">
              {row.plat === null && row.buyPlat === null ? (
                <span className="text-muted-foreground text-sm">no price</span>
              ) : (
                <>
                  <RankedPlat
                    label="WTS"
                    value={row.plat}
                    floor={row.platIsFloor}
                    to={marketListingPath(row.marketSlug, "sell", row.rank)}
                  />
                  <RankedPlat
                    label="WTB"
                    value={row.buyPlat}
                    tone="text-accent"
                    to={marketListingPath(row.marketSlug, "buy", row.rank)}
                  />
                </>
              )}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}

export const ItemCard = memo(ItemCardInner);
