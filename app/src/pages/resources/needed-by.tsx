import { FavouriteStar } from "@/components/favourite-star";
import { ItemImage } from "@/components/item-image";
import { Section } from "@/components/page";
import { num } from "@/lib/format";
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
      description={`${num(row.used_by.length)} ${row.used_by.length === 1 ? "item needs" : "items need"} ${row.name}`}
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
              <span className="min-w-0 flex-1 truncate">{use.name}</span>
              {use.ready_to_build && (
                <span className="text-accent shrink-0 text-xs">
                  Ready to build
                </span>
              )}
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
