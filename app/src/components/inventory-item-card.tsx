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
import type { InventoryTabKey } from "@/lib/inventory-filters";
import { equippedLabel, type Row } from "@/lib/inventory-rows";
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

function SetParts({ set }: { set: SetRow }) {
  const openListing = useMarketPanelStore((state) => state.openListing);
  const side = useAppStore((state) => state.settings?.set_part_click ?? "sell");
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex flex-wrap items-center gap-1">
        {set.components.map((part) => (
          <button
            key={part.unique_name}
            type="button"
            onClick={() => openListing(part.market_slug, side)}
            title={`${part.name}: ${part.owned}/${part.required}. Open on warframe.market`}
            className={cn(
              "hover:border-accent cursor-pointer rounded-md border-2 hover:opacity-100 hover:grayscale-0",
              part.enough
                ? "border-primary"
                : "border-transparent opacity-35 grayscale",
            )}
          >
            <ItemImage imageName={part.image_name} size={26} />
          </button>
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

function listingStars(row: Row) {
  return row.stars
    ? { amber: row.stars.amber_filled, cyan: row.stars.cyan_filled }
    : null;
}

function ItemCardInner({ row, tab }: { row: Row; tab: InventoryTabKey }) {
  const openListing = useMarketPanelStore((state) => state.openListing);
  const isSet = tab === "sets" && row.set;
  const equipped = equippedLabel(row);
  const crafted = (tab === "parts" || tab === "sets") && row.itemOwned;
  const stars = listingStars(row);

  return (
    <div className="bg-card hover:border-primary/50 flex gap-4 rounded-xl border p-4 transition-colors">
      <div className="relative shrink-0 self-start">
        <ArcaneImage
          imageName={row.imageName}
          rarity={tab === "arcanes" ? row.rarity : null}
          size={isSet ? 96 : 80}
          alt={row.name}
          className={cn((tab === "mods" || tab === "arcanes") && "bg-muted/60")}
        />
        {(!isSet || row.count > 0) && (
          <ImageTag className="top-1 right-1">x{num(row.count)}</ImageTag>
        )}
        {row.ducats ? (
          <ImageTag className="text-accent bottom-1 left-1/2 -translate-x-1/2">
            {num(row.ducats)}
            <GameIcon name="ducats" size={14} alt="Ducats" />
          </ImageTag>
        ) : null}
      </div>
      <div className="flex min-w-0 flex-1 flex-col gap-1.5">
        <div className="flex items-start justify-between gap-2">
          <span className="flex min-w-0 items-center gap-1.5">
            <span
              className="truncate text-sm leading-tight font-semibold"
              title={row.name}
            >
              {row.name}
            </span>
            {row.rank !== null && (
              <span className="text-muted-foreground shrink-0 text-sm">
                R{row.rank}
              </span>
            )}
            {isSet && row.mastered && (
              <GameIcon name="mastered" size={16} alt="Mastered" />
            )}
          </span>
          <span className="-mt-1 -mr-1 flex shrink-0 items-center">
            <FavouriteStar
              uniqueName={row.uniqueName}
              favourite={row.favourite}
            />
          </span>
        </div>
        <div className="text-muted-foreground flex flex-wrap items-center gap-x-3 gap-y-0.5 text-sm tabular-nums">
          {row.refinement && (
            <Meta className={cn("font-medium", refinementTone(row.refinement))}>
              {row.tier && (
                <GameIcon
                  name={relicTierIcon(row.tier)}
                  size={14}
                  alt={row.tier}
                />
              )}
              {row.refinement}
            </Meta>
          )}
          {row.stars && (
            <Meta
              className={cn(
                row.stars.filled === row.stars.amber + row.stars.cyan &&
                  "text-accent",
              )}
            >
              {row.stars.filled}/{row.stars.amber + row.stars.cyan} stars
            </Meta>
          )}
          {row.vault === "vaulted" && (
            <Meta className="text-vaulted">
              <Archive className="size-3.5" />
              Vaulted
            </Meta>
          )}
          {crafted && (
            <Meta className="text-accent">
              <CircleCheck className="size-3.5" />
              Crafted
            </Meta>
          )}
          {isSet && row.setComplete && (
            <Meta className="text-accent">
              <CircleCheck className="size-3.5" />
              Complete
            </Meta>
          )}
          {row.orders.sell && (
            <Meta className="text-primary">
              <Tag className="size-3.5" />
              Selling
            </Meta>
          )}
          {row.setOrders?.sell && (
            <Meta className="text-primary">
              <Tag className="size-3.5" />
              Selling in set
            </Meta>
          )}
          {row.orders.buy && (
            <Meta className="text-primary">
              <Tag className="size-3.5" />
              Buying
            </Meta>
          )}
          {row.setOrders?.buy && (
            <Meta className="text-primary">
              <Tag className="size-3.5" />
              Buying in set
            </Meta>
          )}
        </div>
        {equipped && <EquippedDialog row={row} label={equipped} />}
        {isSet && row.set && <SetParts set={row.set} />}
        {row.marketSlug && (
          <div className="mt-auto flex flex-wrap items-center justify-end gap-2 pt-1.5">
            {tab === "arcanes" ? (
              <>
                <RankedPlat
                  side="sell"
                  label="R0"
                  value={row.plat}
                  floor={row.platIsFloor}
                  onOpen={() => openListing(row.marketSlug, "sell", 0)}
                />
                {(row.maxRank ?? 0) > 0 && (
                  <RankedPlat
                    side="sell"
                    label={`R${row.maxRank}`}
                    value={row.platMaxRank}
                    onOpen={() =>
                      openListing(row.marketSlug, "sell", row.maxRank)
                    }
                  />
                )}
              </>
            ) : (
              <>
                <RankedPlat
                  side="sell"
                  value={row.plat}
                  floor={row.platIsFloor}
                  onOpen={() =>
                    openListing(
                      row.marketSlug,
                      "sell",
                      row.rank,
                      row.marketSubtype,
                      stars,
                    )
                  }
                />
                <RankedPlat
                  side="buy"
                  value={row.buyPlat}
                  tone="text-accent"
                  onOpen={() =>
                    openListing(
                      row.marketSlug,
                      "buy",
                      row.rank,
                      row.marketSubtype,
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
