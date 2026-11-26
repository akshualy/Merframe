import { FavouriteStar } from "@/components/favourite-star";
import { ItemImage } from "@/components/item-image";
import { MasteryRequirement } from "@/components/mastery-requirement";
import { Section } from "@/components/page";
import { counted, num } from "@/lib/format";
import type { ResourceRow } from "@/types";

export function NeededBy({
  row,
  onOpen,
}: {
  row: ResourceRow;
  onOpen: (uniqueName: string) => void;
}) {
  return (
    <Section
      title="Needed By"
      description={`${counted(row.used_by.length, "item needs", "items need")} ${row.name}`}
      className="self-start @3xl:sticky @3xl:top-4"
    >
      <ul className="flex max-h-128 flex-col gap-1 overflow-y-auto">
        {row.used_by.map((use) => (
          <li key={use.unique_name} className="flex items-center gap-1">
            <FavouriteStar
              uniqueName={use.unique_name}
              favourite={use.favourite}
              size="small"
            />
            <button
              type="button"
              onClick={() => onOpen(use.unique_name)}
              className="hover:bg-secondary/60 flex min-w-0 flex-1 cursor-pointer items-center gap-2 rounded-md px-2 py-1 text-left text-sm"
            >
              <ItemImage
                imageName={use.image_name}
                size={32}
                alt={use.name}
                className="bg-transparent"
              />
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="truncate">{use.name}</span>
                <span className="flex flex-wrap items-center gap-x-2 text-xs empty:hidden">
                  <MasteryRequirement gate={use.mastery} unmetOnly />
                  {use.ready_to_build && (
                    <span className="text-accent">Ready to build</span>
                  )}
                </span>
              </span>
              <span className="text-muted-foreground shrink-0 tabular-nums">
                {num(use.amount)}
              </span>
            </button>
          </li>
        ))}
      </ul>
    </Section>
  );
}
