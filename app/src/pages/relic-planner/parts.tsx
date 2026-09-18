import { X } from "lucide-react";
import { type Dispatch, type SetStateAction, useState } from "react";
import { ItemImage, prefetchImages } from "@/components/item-image";
import { Section } from "@/components/page";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useLoaded } from "@/hooks/use-loaded";
import { api, reportError } from "@/lib/bridge";
import { displayNameFromPath, num, percent } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { MissingPart } from "@/types";

export function WantedParts({
  missing,
  wanted,
  setWanted,
  selected,
  onSelect,
}: {
  missing: MissingPart[];
  wanted: string[];
  setWanted: Dispatch<SetStateAction<string[]>>;
  selected: string | null;
  onSelect: (path: string) => void;
}) {
  const [query, setQuery] = useState("");
  const search = query.trim().toLowerCase();
  const candidates = missing
    .map((part) => ({
      path: part.unique_name,
      name: displayNameFromPath(part.unique_name),
    }))
    .filter((entry) => entry.name.toLowerCase().includes(search))
    .slice(0, 40);
  const masteryParts = missing
    .filter((part) => part.mastery)
    .map((part) => part.unique_name);

  return (
    <Section
      title="Wanted Parts"
      description={`${num(missing.length)} Prime parts missing`}
    >
      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center gap-2">
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search missing parts"
            className="min-w-48 flex-1"
          />
          <Button
            variant="outline"
            disabled={masteryParts.every((path) => wanted.includes(path))}
            onClick={() =>
              setWanted((current) => [
                ...current,
                ...masteryParts.filter((path) => !current.includes(path)),
              ])
            }
          >
            Select Missing for Mastery
          </Button>
        </div>
        {wanted.length > 0 && (
          <div className="flex flex-wrap gap-1.5">
            {wanted.map((path) => (
              <Badge key={path} variant="accent" asChild>
                <button
                  type="button"
                  className="cursor-pointer"
                  onClick={() =>
                    setWanted((current) =>
                      current.filter((entry) => entry !== path),
                    )
                  }
                >
                  {displayNameFromPath(path)}
                  <X className="size-3" />
                </button>
              </Badge>
            ))}
          </div>
        )}
        <ul className="flex max-h-72 flex-col gap-1 overflow-y-auto">
          {candidates.map((entry) => (
            <li key={entry.path}>
              <button
                type="button"
                onClick={() => {
                  onSelect(entry.path);
                  setWanted((current) =>
                    current.includes(entry.path)
                      ? current
                      : [...current, entry.path],
                  );
                }}
                className={cn(
                  "hover:bg-secondary/60 w-full cursor-pointer truncate rounded-md px-2 py-1 text-left text-sm",
                  selected === entry.path && "bg-secondary",
                )}
              >
                {entry.name}
              </button>
            </li>
          ))}
        </ul>
      </div>
    </Section>
  );
}

export function PartSources({ part }: { part: string | null }) {
  const sources =
    useLoaded(
      part,
      async (path) => {
        const relics = await api.relicsFor(path);
        prefetchImages(relics.map((source) => source.image_name));
        return relics;
      },
      (cause) => {
        reportError(cause);
        return [];
      },
    ) ?? [];

  return (
    <Section
      title="Drops From"
      description={part ? displayNameFromPath(part) : "No part selected."}
    >
      <ul className="flex flex-col gap-1.5">
        {sources.length === 0 && (
          <li className="text-muted-foreground text-sm">
            No relic sources loaded.
          </li>
        )}
        {sources.map((source) => (
          <li
            key={`${source.relic}-${source.rarity}`}
            className="flex items-center gap-2 rounded-md border px-2 py-1.5 text-sm"
          >
            <ItemImage
              imageName={source.image_name}
              size={24}
              alt={source.relic}
            />
            <span className="flex-1 truncate">{source.relic}</span>
            <Badge variant="muted">{source.rarity}</Badge>
            <span className="text-muted-foreground w-14 text-right text-xs">
              {percent(source.chance / 100)}
            </span>
            <span className="text-accent w-10 text-right text-xs">
              x{source.owned}
            </span>
          </li>
        ))}
      </ul>
    </Section>
  );
}
