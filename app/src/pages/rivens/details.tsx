import { useState } from "react";
import { GoodRollBlock } from "@/components/good-roll";
import { RivenListings } from "@/components/riven-listings";
import { RivenRollBar } from "@/components/riven-roll-bar";
import { SplicedMark, SpliceOptions } from "@/components/spliced-mark";
import { Hint } from "@/components/ui/hint";
import {
  attributeAmountAtRank,
  attributeLabel,
  attributeValue,
  gradeClasses,
  MAX_RIVEN_RANK,
} from "@/lib/rivens";
import { cn } from "@/lib/utils";
import type { RivenRow } from "@/types";
import { RivenRankSlider } from "./rank-slider";

export function RivenDetails({ riven }: { riven: RivenRow }) {
  const [rank, setRank] = useState(MAX_RIVEN_RANK);

  return (
    <div className="flex flex-col gap-2 py-2">
      <div className="max-w-xs">
        <RivenRankSlider
          id={`rank-${riven.item_id}`}
          label={`Stats at rank ${rank}`}
          rank={rank}
          onRankChange={setRank}
        />
      </div>
      <div className="grid gap-2 @sm:grid-cols-2 @3xl:grid-cols-4">
        {riven.attributes.map((attribute) => (
          <div
            key={attribute.tag}
            className={cn(
              "flex min-w-0 flex-col gap-0.5 rounded-lg border px-3 py-2",
              attribute.curse && "border-warning/40",
            )}
          >
            <span className="flex items-center gap-2 text-sm font-medium">
              <span className="shrink-0">
                {attributeValue(attribute, rank) ??
                  `roll x${attribute.multiplier.toFixed(3)}`}
              </span>
              <span className="truncate">{attributeLabel(attribute)}</span>
              <SplicedMark attribute={attribute} />
              <span
                className={cn(
                  "ml-auto shrink-0 text-xs font-normal",
                  gradeClasses(attribute.grade).text,
                )}
              >
                {attribute.grade}
              </span>
            </span>
            <RivenRollBar attribute={attribute} className="mt-1" />
            {attribute.min !== null && attribute.max !== null && (
              <Hint as="span" className="flex justify-between">
                <span>
                  {attributeAmountAtRank(attribute.unit, attribute.min, rank)}
                </span>
                <span>
                  {attributeAmountAtRank(attribute.unit, attribute.max, rank)}
                </span>
              </Hint>
            )}
          </div>
        ))}
      </div>
      <SpliceOptions splices={riven.splices} />
      {riven.good_roll && <GoodRollBlock view={riven.good_roll} />}
      {riven.weapon_slug && (
        <RivenListings
          weaponSlug={riven.weapon_slug}
          attributes={riven.attributes}
          limit={10}
        />
      )}
    </div>
  );
}
