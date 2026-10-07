import { FavouriteStar } from "@/components/favourite-star";
import { GameIcon } from "@/components/game-icon";
import { ItemImage } from "@/components/item-image";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import { num, percent, plat } from "@/lib/format";
import { REFINEMENTS } from "@/lib/relics";
import { cn } from "@/lib/utils";
import { useMarketPanelStore } from "@/stores/market-panel-store";
import type { RefinementValue, RelicPlan } from "@/types";
import type { PlannerRow, RewardView } from "./columns";

function rarityVariant(
  rarity: string,
): "muted" | "outline" | "secondary" | "accent" {
  switch (rarity) {
    case "Rare":
      return "secondary";
    case "Uncommon":
      return "outline";
    case "Legendary":
      return "accent";
    default:
      return "muted";
  }
}

function RewardRow({ view }: { view: RewardView }) {
  const { reward, chance, squadChance, share, wanted } = view;
  return (
    <li className="flex items-center gap-2 border-t py-1.5 text-sm first:border-t-0">
      <ItemImage imageName={reward.image_name} size={24} alt={reward.name} />
      <FavouriteStar
        uniqueName={reward.unique_name}
        favourite={reward.favourite}
        size="small"
      />
      <span className="flex-1 truncate">{reward.name}</span>
      {wanted && <Badge variant="accent">wanted</Badge>}
      {!wanted && reward.ownership.needed_for_set && (
        <Badge variant="outline">needed</Badge>
      )}
      {!wanted &&
        !reward.ownership.needed_for_set &&
        reward.ownership.owned > 0 && <Badge variant="muted">owned</Badge>}
      <Badge variant={rarityVariant(reward.rarity)} className="w-20">
        {reward.rarity}
      </Badge>
      <span className="w-16 text-right text-xs">
        {percent(chance / 100, 2)}
      </span>
      <span className="w-16 text-right text-xs">
        {percent(squadChance / 100, 2)}
      </span>
      <span className="flex w-16 items-center justify-end gap-0.5 text-xs">
        {num(reward.plat)}
        <GameIcon name="platinum" size={16} alt="Platinum" />
      </span>
      <span className="text-primary flex w-20 items-center justify-end gap-0.5 text-xs font-medium">
        {plat(share)}
        <GameIcon name="platinum" size={16} alt="Platinum" />
      </span>
    </li>
  );
}

function RefinementCard({
  entry,
  chosen,
}: {
  entry: RefinementValue;
  chosen: boolean;
}) {
  const litDots = REFINEMENTS.indexOf(entry.refinement);
  return (
    <span
      title={entry.refinement}
      className={cn(
        "flex h-8 items-center gap-2 rounded-md border px-2.5",
        chosen && "border-primary font-bold",
      )}
    >
      <span className="flex flex-col gap-0.5" aria-hidden="true">
        {REFINEMENTS.slice(1).map((step, dot) => (
          <span
            key={step}
            className={cn(
              "size-1.5 rounded-full",
              dot < litDots ? "bg-primary" : "bg-muted-foreground/40",
            )}
          />
        ))}
      </span>
      <span className="sr-only">{entry.refinement}</span>
      <span className="flex items-center gap-0.5">
        {plat(entry.expected_plat)}
        <GameIcon name="platinum" size={16} alt="Platinum" />
      </span>
      {entry.plat_per_trace !== null && (
        <span className="text-muted-foreground font-normal">
          {entry.plat_per_trace.toFixed(2)}/trace
        </span>
      )}
    </span>
  );
}

function RelicValues({ row }: { row: PlannerRow }) {
  const openListing = useMarketPanelStore((state) => state.openListing);
  const { plan } = row;
  const { market } = plan;
  return (
    <div className="flex flex-wrap items-center gap-2 text-xs">
      {market && (
        <>
          <Button
            variant="outline"
            size="sm"
            onClick={() => openListing(market.slug, "sell")}
          >
            Sell {num(market.sell)}
            <GameIcon name="platinum" size={16} alt="Platinum" />
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => openListing(market.slug, "buy")}
          >
            Buy {num(market.buy)}
            <GameIcon name="platinum" size={16} alt="Platinum" />
          </Button>
        </>
      )}
      {plan.values.map((entry) => (
        <RefinementCard
          key={entry.refinement}
          entry={entry}
          chosen={entry.refinement === row.value.refinement}
        />
      ))}
    </div>
  );
}

function DropLocations({ plan }: { plan: RelicPlan }) {
  if (plan.drops.length === 0) {
    return <Hint>This relic does not drop anywhere right now.</Hint>;
  }
  const rest = plan.drop_locations - plan.drops.length;
  return (
    <div className="flex flex-col gap-1">
      <Hint as="span">
        Drop locations
        {rest > 0
          ? `, best ${plan.drops.length} of ${plan.drop_locations}`
          : ""}
      </Hint>
      <ul className="flex flex-col">
        {plan.drops.map((drop) => (
          <li
            key={drop.location}
            className="flex items-center gap-2 border-t py-1 text-xs first:border-t-0"
          >
            <span className="flex-1 truncate">{drop.location}</span>
            <span className="w-16 text-right">
              {percent(drop.chance / 100, 2)}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}

export function PlannerRowDetails({ row }: { row: PlannerRow }) {
  return (
    <div className="flex flex-col gap-2 py-2">
      <RelicValues row={row} />
      <ul className="flex flex-col">
        <li className="text-muted-foreground flex items-center gap-2 pb-1 text-xs">
          <span className="w-6" />
          <span className="w-6" />
          <span className="flex-1" />
          <span className="w-20">Rarity</span>
          <span className="w-16 text-right">Chance</span>
          <span className="w-16 text-right">In squad</span>
          <span className="w-16 text-right">Price</span>
          <span className="w-20 text-right">Of expected</span>
        </li>
        {row.rewards.map((view) => (
          <RewardRow key={view.reward.unique_name} view={view} />
        ))}
      </ul>
      <DropLocations plan={row.plan} />
    </div>
  );
}
