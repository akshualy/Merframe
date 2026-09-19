import { useCallback, useEffect, useMemo, useState } from "react";
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
import { NodeDetails, WikiButton } from "./node-details";

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
  onSelect,
}: {
  label: string;
  items: NeededItem[];
  onSelect: (uniqueName: string) => void;
}) {
  if (items.length === 0) {
    return null;
  }
  return (
    <div className="flex flex-col gap-1.5">
      <Hint as="span">{label}</Hint>
      <div className="flex flex-wrap gap-2">
        {items.map((needed) => (
          <Badge key={needed.unique_name} variant="secondary" asChild>
            <button
              type="button"
              onClick={() => onSelect(needed.unique_name)}
              className="cursor-pointer"
            >
              <ItemImage
                imageName={needed.image_name}
                size={18}
                alt={needed.name}
              />
              {needed.name}
              <span className="tabular-nums">x{num(needed.amount)}</span>
            </button>
          </Badge>
        ))}
      </div>
    </div>
  );
}

function childPath(prefix: string, position: number): string {
  return prefix === "" ? String(position) : `${prefix}.${position}`;
}

function defaultPath(nodes: CraftNode[]): string | null {
  if (nodes.length === 0) {
    return null;
  }
  return String(
    Math.max(
      0,
      nodes.findIndex((node) => !node.stocked),
    ),
  );
}

function pathOf(
  nodes: CraftNode[],
  uniqueName: string,
  prefix = "",
): string | null {
  for (const [position, node] of nodes.entries()) {
    const path = childPath(prefix, position);
    if (node.unique_name === uniqueName && !node.stocked) {
      return path;
    }
    const deeper = pathOf(node.children, uniqueName, path);
    if (deeper !== null) {
      return deeper;
    }
  }
  return null;
}

function nodeAt(nodes: CraftNode[], path: string): CraftNode | null {
  let level = nodes;
  let found: CraftNode | null = null;
  for (const step of path.split(".")) {
    found = level[Number(step)] ?? null;
    if (!found) {
      return null;
    }
    level = found.children;
  }
  return found;
}

function Tree({
  nodes,
  hideCompleted,
  selected,
  onSelect,
  prefix = "",
  depth = 0,
}: {
  nodes: CraftNode[];
  hideCompleted: boolean;
  selected: string | null;
  onSelect: (path: string) => void;
  prefix?: string;
  depth?: number;
}) {
  const keys = occurrenceKeys(nodes.map((node) => node.unique_name));
  return (
    <ul className={cn("flex flex-col gap-1", depth > 0 && "border-l pl-4")}>
      {nodes.map((node, position) => {
        const path = childPath(prefix, position);
        return (
          <li key={keys[position]} className="flex flex-col gap-1">
            <button
              type="button"
              onClick={() => onSelect(path)}
              className={cn(
                "flex w-fit cursor-pointer items-center gap-2 rounded-md px-1.5 py-0.5 text-left text-sm",
                selected === path
                  ? "bg-foreground/10"
                  : "hover:bg-foreground/5",
              )}
            >
              <ItemImage
                imageName={node.image_name}
                size={22}
                alt={node.name}
              />
              <span
                className={cn(
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
            </button>
            {node.children.length > 0 && !(hideCompleted && node.stocked) && (
              <Tree
                nodes={node.children}
                hideCompleted={hideCompleted}
                selected={selected}
                onSelect={onSelect}
                prefix={path}
                depth={depth + 1}
              />
            )}
          </li>
        );
      })}
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
  const [selected, setSelected] = useState<string | null>(null);

  useEffect(() => {
    if (!item) {
      return;
    }
    let cancelled = false;
    setDetails(null);
    setSelected(null);
    async function load(uniqueName: string) {
      try {
        const next = await api.craftTree(uniqueName);
        if (!cancelled) {
          setDetails(next);
          setSelected(defaultPath(next.tree));
        }
      } catch (error) {
        if (!cancelled) {
          setDetails({ tree: [], summary: EMPTY_SUMMARY });
          reportError(error);
        }
      }
    }
    load(item.unique_name);
    return () => {
      cancelled = true;
    };
  }, [item]);

  const selectedNode = useMemo(
    () => (details && selected ? nodeAt(details.tree, selected) : null),
    [details, selected],
  );

  const selectNeeded = useCallback(
    (uniqueName: string) =>
      setSelected((current) =>
        details ? (pathOf(details.tree, uniqueName) ?? current) : current,
      ),
    [details],
  );

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
            {item?.wiki_url && <WikiButton url={item.wiki_url} />}
          </DialogTitle>
          {details && details.tree.length > 0 && (
            <div className="flex flex-wrap items-center justify-between gap-3">
              <Summary summary={details.summary} />
              <CheckboxField
                id="hideCompleted"
                checked={hideCompleted}
                onChange={setHideCompleted}
              >
                Collapse parts you already own
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
              <Tree
                nodes={details.tree}
                hideCompleted={hideCompleted}
                selected={selected}
                onSelect={setSelected}
              />
              {selectedNode && <NodeDetails node={selectedNode} />}
              <ShoppingList
                label="Blueprints still to craft"
                items={details.summary.blueprints_needed}
                onSelect={selectNeeded}
              />
              <ShoppingList
                label="Resources still to gather"
                items={details.summary.resources_needed}
                onSelect={selectNeeded}
              />
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
