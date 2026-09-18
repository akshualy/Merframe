import { useEffect, useState } from "react";
import { FavouriteStar } from "@/components/favourite-star";
import { ItemImage } from "@/components/item-image";
import { EmptyNote } from "@/components/page";
import { Badge } from "@/components/ui/badge";
import { CheckboxField } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Hint } from "@/components/ui/hint";
import { Skeleton } from "@/components/ui/skeleton";
import { api, reportError } from "@/lib/bridge";
import { countdown, num } from "@/lib/format";
import { occurrenceKeys } from "@/lib/keys";
import { cn } from "@/lib/utils";
import type {
  CraftDetails,
  CraftNode,
  CraftSummary,
  FoundryItem,
  NeededItem,
} from "@/types";

const EMPTY_SUMMARY: CraftSummary = {
  credits: 0,
  build_secs: 0,
  shortest_secs: 0,
  blueprints_needed: [],
  resources_needed: [],
};

function duration(seconds: number): string {
  return seconds > 0 ? countdown(seconds) : "-";
}

function Summary({ summary }: { summary: CraftSummary }) {
  const fields: [string, string][] = [
    ["Credits", num(summary.credits)],
    ["Build time", duration(summary.build_secs)],
    ["Shortest time", duration(summary.shortest_secs)],
  ];
  return (
    <div className="text-muted-foreground flex flex-wrap items-baseline gap-x-4 gap-y-1 text-xs">
      {fields.map(([label, value]) => (
        <span key={label} className="flex items-baseline gap-1.5">
          {label}
          <span className="text-foreground tabular-nums">{value}</span>
        </span>
      ))}
    </div>
  );
}

function ShoppingList({
  label,
  items,
}: {
  label: string;
  items: NeededItem[];
}) {
  if (items.length === 0) {
    return null;
  }
  return (
    <div className="flex flex-col gap-1.5">
      <Hint as="span">{label}</Hint>
      <div className="flex flex-wrap gap-2">
        {items.map((needed) => (
          <Badge key={needed.unique_name} variant="secondary">
            <ItemImage
              imageName={needed.image_name}
              size={18}
              alt={needed.name}
            />
            {needed.name}
            <span className="tabular-nums">x{num(needed.amount)}</span>
          </Badge>
        ))}
      </div>
    </div>
  );
}

function Tree({
  nodes,
  hideCompleted,
  depth = 0,
}: {
  nodes: CraftNode[];
  hideCompleted: boolean;
  depth?: number;
}) {
  const keys = occurrenceKeys(nodes.map((node) => node.unique_name));
  return (
    <ul className={cn("flex flex-col gap-1", depth > 0 && "border-l pl-4")}>
      {nodes.map((node, position) => (
        <li key={keys[position]} className="flex flex-col gap-1">
          <span className="flex items-center gap-2 text-sm">
            <ItemImage imageName={node.image_name} size={22} alt={node.name} />
            <span
              className={cn(
                "font-medium",
                node.covered && "opacity-50",
                node.stocked ? "text-foreground" : "text-warning",
              )}
            >
              {node.name}
            </span>
            <Hint as="span" className="tabular-nums">
              {num(node.owned)} / {num(node.required)}
            </Hint>
            {node.craftable && <Badge variant="accent">craftable</Badge>}
          </span>
          {node.children.length > 0 && !(hideCompleted && node.stocked) && (
            <Tree
              nodes={node.children}
              hideCompleted={hideCompleted}
              depth={depth + 1}
            />
          )}
        </li>
      ))}
    </ul>
  );
}

export function FoundryTreeDialog({
  item,
  onClose,
}: {
  item: FoundryItem | null;
  onClose: () => void;
}) {
  const [details, setDetails] = useState<CraftDetails | null>(null);
  const [hideCompleted, setHideCompleted] = useState(true);

  useEffect(() => {
    if (!item) {
      return;
    }
    let cancelled = false;
    setDetails(null);
    async function load(uniqueName: string) {
      try {
        const next = await api.craftTree(uniqueName);
        if (!cancelled) {
          setDetails(next);
        }
      } catch (error) {
        if (!cancelled) {
          setDetails({ tree: [], missing: [], summary: EMPTY_SUMMARY });
          reportError(error);
        }
      }
    }
    load(item.unique_name);
    return () => {
      cancelled = true;
    };
  }, [item]);

  return (
    <Dialog open={item !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="top-16 max-h-[calc(100vh-8rem)] translate-y-0 overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-3">
            <ItemImage imageName={item?.image_name} size={40} />
            {item?.name}
            {item && (
              <FavouriteStar
                uniqueName={item.unique_name}
                favourite={item.favourite}
              />
            )}
          </DialogTitle>
          <DialogDescription>
            Required parts and current stock
          </DialogDescription>
          {details && details.tree.length > 0 && (
            <div className="flex flex-wrap items-center justify-between gap-3">
              <Summary summary={details.summary} />
              <CheckboxField
                id="hideCompleted"
                checked={hideCompleted}
                onChange={setHideCompleted}
              >
                Hide what you already own
              </CheckboxField>
            </div>
          )}
        </DialogHeader>
        <div className="min-h-40">
          {details === null ? (
            <Skeleton className="h-40 w-full" />
          ) : details.tree.length === 0 ? (
            <EmptyNote>This item has no recipe.</EmptyNote>
          ) : (
            <div className="flex flex-col gap-4">
              <Tree nodes={details.tree} hideCompleted={hideCompleted} />
              <ShoppingList
                label="Blueprints still to craft"
                items={details.summary.blueprints_needed}
              />
              <ShoppingList
                label="Resources still to gather"
                items={details.summary.resources_needed}
              />
              {details.missing.length > 0 && (
                <div className="flex flex-col gap-1.5">
                  <Hint as="span">Short for this recipe</Hint>
                  <div className="flex flex-wrap gap-2">
                    {details.missing.map((component) => (
                      <Badge key={component.unique_name} variant="warning">
                        <ItemImage
                          imageName={component.image_name}
                          size={18}
                          alt={component.name}
                        />
                        {component.name} {num(component.owned)}/
                        {num(component.required)}
                      </Badge>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
