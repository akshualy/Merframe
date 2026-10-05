import { Archive, CircleCheck, Receipt, ShoppingCart, Tag } from "lucide-react";
import { memo, type ReactNode } from "react";
import { EquippedDialog } from "@/components/equipped-dialog";
import { FavouriteStar } from "@/components/favourite-star";
import { GameIcon, relicTierIcon } from "@/components/game-icon";
import { ArcaneImage, ItemImage } from "@/components/item-image";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import { Progress } from "@/components/ui/progress";
import { num } from "@/lib/format";
import {
  buyPlat,
  crafted,
  ducats,
  equippedIn,
  equippedLabel,
  type InventoryEntry,
  type InventoryFlags,
  rank,
  refinement,
  type SetPart,
  sellPlat,
  vault,
} from "@/lib/inventory-entries";
import type { InventoryTabKey } from "@/lib/inventory-filters";
import { refinementTone } from "@/lib/relics";
import { cn } from "@/lib/utils";
import { useAppStore } from "@/stores/app-store";
import { useMarketPanelStore } from "@/stores/market-panel-store";
import type { OrderType, SetRow } from "@/types";

function RankedPlat({
  side,
  label,
  value,
  floor,
  tone,
  onOpen,
}: {
  side: OrderType;
  label?: string;
  value: number | null;
  floor?: boolean;
  tone?: string;
  onOpen?: () => void;
}) {
  const body = (
    <>
      <Hint as="span">
        {label ??
          (side === "sell" ? (
            <Receipt className="size-4" />
          ) : (
            <ShoppingCart className="size-4" />
          ))}
      </Hint>
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
  if (onOpen) {
    return (
      <Button
        variant="outline"
        size="sm"
        className="h-8 max-w-36 basis-20 grow justify-between gap-2 px-3"
        onClick={onOpen}
        title={`${side === "buy" ? "Buy" : "Sell"} on warframe.market`}
      >
        {body}
      </Button>
    );
  }
  return <span className="flex items-baseline gap-1 text-xs">{body}</span>;
}

function SetParts({ set, parts }: { set: SetRow; parts: SetPart[] }) {
  const openListing = useMarketPanelStore((state) => state.openListing);
  const side = useAppStore((state) => state.settings?.set_part_click ?? "sell");
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex flex-wrap items-center gap-1">
        {parts.map(({ component, item }) => {
          const slug = item.market_slug;
          return (
            <button
              key={item.unique_name}
              type="button"
              disabled={slug === null}
              onClick={slug ? () => openListing(slug, side) : undefined}
              title={`${item.name}: ${component.owned}/${component.required}. Open on warframe.market`}
              className={cn(
                "hover:border-accent cursor-pointer rounded-md border-2 hover:opacity-100 hover:grayscale-0",
                component.enough
                  ? "border-primary"
                  : "border-transparent opacity-35 grayscale",
              )}
            >
              <ItemImage imageName={item.image_name} size={26} />
            </button>
          );
        })}
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

function ImageTag({
  className,
  children,
}: {
  className: string;
  children: ReactNode;
}) {
  return (
    <span
      className={cn(
        "bg-background/80 absolute flex items-center gap-0.5 rounded-md px-1 text-xs font-medium tabular-nums",
        className,
      )}
    >
      {children}
    </span>
  );
}

function Meta({
  className,
  children,
}: {
  className?: string;
  children: ReactNode;
}) {
  return (
    <span className={cn("flex items-center gap-1", className)}>{children}</span>
  );
}

function marketSubtype(entry: InventoryEntry): string | null {
  switch (entry.kind) {
    case "relic":
      return entry.row.refinement.toLowerCase();
    case "misc":
      return entry.row.market_subtype;
    default:
      return null;
  }
}

function ItemCardInner({
  entry,
  tab,
  flags,
}: {
  entry: InventoryEntry;
  tab: InventoryTabKey;
  flags: InventoryFlags;
}) {
  const openListing = useMarketPanelStore((state) => state.openListing);
  const { item, row } = entry;
  const slug = item.market_slug;
  const set = entry.kind === "set" ? entry : null;
  const upgrade = entry.kind === "upgrade" ? entry.row : null;
  const sculpture = entry.kind === "misc" ? entry.row.stars : null;
  const tier = entry.kind === "relic" ? entry.row.tier : null;
  const setItem = entry.kind === "part" ? entry.row.set.item : null;
  const holders = equippedIn(entry);
  const equipped = equippedLabel(holders);
  const itemRank = rank(entry);
  const itemRefinement = refinement(entry);
  const itemDucats = ducats(entry);
  const stars = sculpture
    ? { amber: sculpture.amber_filled, cyan: sculpture.cyan_filled }
    : null;
  const starsFilled = sculpture
    ? sculpture.amber_filled + sculpture.cyan_filled
    : 0;
  const starSockets = sculpture
    ? sculpture.amber_sockets + sculpture.cyan_sockets
    : 0;

  return (
    <div className="bg-card hover:border-primary/50 flex gap-4 rounded-xl border p-4 transition-colors">
      <div className="relative shrink-0 self-start">
        <ArcaneImage
          imageName={item.image_name}
          rarity={tab === "arcanes" ? (upgrade?.rarity ?? null) : null}
          size={set ? 96 : 80}
          alt={item.name}
          className={cn((tab === "mods" || tab === "arcanes") && "bg-muted/60")}
        />
        {(!set || row.count > 0) && (
          <ImageTag className="top-1 right-1">x{num(row.count)}</ImageTag>
        )}
        {itemDucats ? (
          <ImageTag className="text-accent bottom-1 left-1/2 -translate-x-1/2">
            {num(itemDucats)}
            <GameIcon name="ducats" size={14} alt="Ducats" />
          </ImageTag>
        ) : null}
      </div>
      <div className="flex min-w-0 flex-1 flex-col gap-1.5">
        <div className="flex items-start justify-between gap-2">
          <span className="flex min-w-0 items-center gap-1.5">
            <span
              className="truncate text-sm leading-tight font-semibold"
              title={item.name}
            >
              {item.name}
            </span>
            {itemRank !== null && (
              <span className="text-muted-foreground shrink-0 text-sm">
                R{itemRank}
              </span>
            )}
            {set?.row.status.mastered && (
              <GameIcon name="mastered" size={16} alt="Mastered" />
            )}
          </span>
          <span className="-mt-1 -mr-1 flex shrink-0 items-center">
            <FavouriteStar
              uniqueName={item.unique_name}
              favourite={flags.favourites.has(row.item)}
            />
          </span>
        </div>
        <div className="text-muted-foreground flex flex-wrap items-center gap-x-3 gap-y-0.5 text-sm tabular-nums">
          {itemRefinement && (
            <Meta className={cn("font-medium", refinementTone(itemRefinement))}>
              {tier && (
                <GameIcon name={relicTierIcon(tier)} size={14} alt={tier} />
              )}
              {itemRefinement}
            </Meta>
          )}
          {sculpture && (
            <Meta className={cn(starsFilled === starSockets && "text-accent")}>
              {starsFilled}/{starSockets} stars
            </Meta>
          )}
          {vault(entry) === "vaulted" && (
            <Meta className="text-vaulted">
              <Archive className="size-3.5" />
              Vaulted
            </Meta>
          )}
          {crafted(entry) && (
            <Meta className="text-accent">
              <CircleCheck className="size-3.5" />
              Crafted
            </Meta>
          )}
          {set?.row.complete && (
            <Meta className="text-accent">
              <CircleCheck className="size-3.5" />
              Complete
            </Meta>
          )}
          {flags.selling.has(row.item) && (
            <Meta className="text-primary">
              <Tag className="size-3.5" />
              Selling
            </Meta>
          )}
          {setItem !== null && flags.selling.has(setItem) && (
            <Meta className="text-primary">
              <Tag className="size-3.5" />
              Selling in set
            </Meta>
          )}
          {flags.buying.has(row.item) && (
            <Meta className="text-primary">
              <Tag className="size-3.5" />
              Buying
            </Meta>
          )}
          {setItem !== null && flags.buying.has(setItem) && (
            <Meta className="text-primary">
              <Tag className="size-3.5" />
              Buying in set
            </Meta>
          )}
        </div>
        {equipped && (
          <EquippedDialog
            name={item.name}
            rank={itemRank}
            holders={holders}
            label={equipped}
          />
        )}
        {set && <SetParts set={set.row} parts={set.parts} />}
        {slug && (
          <div className="mt-auto flex flex-wrap items-center justify-end gap-2 pt-1.5">
            {upgrade && tab === "arcanes" ? (
              <>
                <RankedPlat
                  side="sell"
                  label="R0"
                  value={upgrade.prices.sell}
                  floor={upgrade.prices.is_floor}
                  onOpen={() => openListing(slug, "sell", 0)}
                />
                {(upgrade.max_rank ?? 0) > 0 && (
                  <RankedPlat
                    side="sell"
                    label={`R${upgrade.max_rank}`}
                    value={upgrade.prices.sell_max_rank}
                    onOpen={() => openListing(slug, "sell", upgrade.max_rank)}
                  />
                )}
              </>
            ) : (
              <>
                <RankedPlat
                  side="sell"
                  value={sellPlat(entry)}
                  floor={upgrade?.prices.is_floor}
                  onOpen={() =>
                    openListing(
                      slug,
                      "sell",
                      itemRank,
                      marketSubtype(entry),
                      stars,
                    )
                  }
                />
                <RankedPlat
                  side="buy"
                  value={buyPlat(entry)}
                  tone="text-accent"
                  onOpen={() =>
                    openListing(
                      slug,
                      "buy",
                      itemRank,
                      marketSubtype(entry),
                      stars,
                    )
                  }
                />
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

export const ItemCard = memo(ItemCardInner);
