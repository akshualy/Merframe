import { ItemImage } from "@/components/item-image";
import { Surface } from "@/components/page";
import { Hint } from "@/components/ui/hint";
import { displayNameFromPath, num } from "@/lib/format";
import type { VeiledGroup } from "@/types";

export function VeiledChallenge({ group }: { group: VeiledGroup }) {
  return (
    <Surface className="p-3">
      <div className="flex flex-wrap items-baseline justify-between gap-2">
        <span className="text-sm font-semibold">{group.challenge}</span>
        <Hint as="span" className="tabular-nums">
          {num(group.count)} veiled
        </Hint>
      </div>
      {group.complication && (
        <Hint as="span">Complication: {group.complication}</Hint>
      )}
      <div className="flex flex-wrap gap-2">
        {group.rivens.map((riven) => (
          <span
            key={riven.riven_id}
            className="flex items-center gap-2 rounded-lg border px-2 py-1"
          >
            <ItemImage
              imageName={riven.image_name}
              size={26}
              alt={riven.weapon_class ?? "Veiled riven"}
            />
            <span className="flex flex-col">
              <span className="text-xs font-medium">
                {riven.weapon_class ?? displayNameFromPath(riven.item_type)}
              </span>
              <Hint as="span" className="tabular-nums">
                {riven.count > 1 && `x${num(riven.count)}`}
                {riven.required > 0 &&
                  ` ${num(riven.progress)} of ${num(riven.required)}`}
              </Hint>
            </span>
          </span>
        ))}
      </div>
    </Surface>
  );
}
