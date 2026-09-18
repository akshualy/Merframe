import { GoodRollBlock } from "@/components/good-roll";
import { RivenListings } from "@/components/riven-listings";
import { Hint } from "@/components/ui/hint";
import { percent } from "@/lib/format";
import { attributeRange, attributeValue, gradeTone } from "@/lib/rivens";
import { cn } from "@/lib/utils";
import type { RivenRow } from "@/types";

export function RivenDetails({ riven }: { riven: RivenRow }) {
  return (
    <div className="flex flex-col gap-2 py-2">
      <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
        {riven.attributes.map((attribute) => {
          const range = attributeRange(attribute);
          return (
            <div
              key={attribute.tag}
              className={cn(
                "flex flex-col gap-0.5 rounded-lg border px-3 py-2",
                attribute.curse && "border-warning/40",
              )}
            >
              <span className="text-sm font-medium">
                {attribute.name ?? attribute.tag}
              </span>
              <Hint as="span">
                {attributeValue(attribute) ??
                  `roll x${attribute.multiplier.toFixed(3)}`}
                {range === null ? "" : ` of ${range}`}
              </Hint>
              <span className="flex items-center gap-1 text-xs">
                <span className="text-accent">
                  {percent(attribute.percentile)} of range
                </span>
                <span className={cn("font-mono", gradeTone(attribute.grade))}>
                  {attribute.grade}
                </span>
              </span>
            </div>
          );
        })}
      </div>
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
