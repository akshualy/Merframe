import { Plus, X } from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { DateTimePicker } from "@/components/ui/date-time-picker";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { api, logError } from "@/lib/bridge";
import type { StoredTrade, Trade, TradeItem } from "@/types";

interface ItemDraft {
  key: number;
  name: string;
  count: string;
  rank: string;
}

interface TradeDraft {
  at: Date;
  partner: string;
  direction: "received" | "paid";
  plat: string;
  offered: ItemDraft[];
  received: ItemDraft[];
}

export interface TradeEntry {
  atMs: number;
  partner: string | null;
  trade: Trade;
}

const ITEM_NAMES_ID = "trade-item-names";

let nextKey = 0;

function itemDraft(item?: TradeItem): ItemDraft {
  nextKey += 1;
  return {
    key: nextKey,
    name: item?.name ?? "",
    count: String(item?.count ?? 1),
    rank: item?.rank == null ? "" : String(item.rank),
  };
}

function draftOf(stored: StoredTrade | null): TradeDraft {
  if (!stored) {
    return {
      at: new Date(),
      partner: "",
      direction: "received",
      plat: "0",
      offered: [itemDraft()],
      received: [],
    };
  }
  return {
    at: new Date(stored.at),
    partner: stored.partner ?? "",
    direction: stored.trade.plat < 0 ? "paid" : "received",
    plat: String(Math.abs(stored.trade.plat)),
    offered: stored.trade.offered.map(itemDraft),
    received: stored.trade.received.map(itemDraft),
  };
}

function wholeNumber(value: string): number | null {
  const parsed = Number.parseInt(value, 10);
  return Number.isInteger(parsed) &&
    parsed >= 0 &&
    String(parsed) === value.trim()
    ? parsed
    : null;
}

function itemOf(draft: ItemDraft): TradeItem | null {
  const name = draft.name.trim();
  const count = wholeNumber(draft.count);
  const unranked = draft.rank.trim() === "";
  const rank = unranked ? null : wholeNumber(draft.rank);
  if (name === "" || !count || (!unranked && rank === null)) {
    return null;
  }
  return { name, count, rank };
}

function entryOf(draft: TradeDraft): TradeEntry | null {
  const atMs = draft.at.getTime();
  const plat = wholeNumber(draft.plat);
  const offered = draft.offered.map(itemOf);
  const received = draft.received.map(itemOf);
  if (
    plat === null ||
    offered.some((item) => item === null) ||
    received.some((item) => item === null)
  ) {
    return null;
  }
  const items = {
    offered: offered.filter((item) => item !== null),
    received: received.filter((item) => item !== null),
  };
  if (plat === 0 && items.offered.length + items.received.length === 0) {
    return null;
  }
  const partner = draft.partner.trim();
  return {
    atMs,
    partner: partner === "" ? null : partner,
    trade: {
      ...items,
      plat: draft.direction === "paid" ? -plat : plat,
    },
  };
}

function ItemRows({
  label,
  items,
  onChange,
}: {
  label: string;
  items: ItemDraft[];
  onChange: (items: ItemDraft[]) => void;
}) {
  const update = (key: number, patch: Partial<ItemDraft>) =>
    onChange(
      items.map((item) => (item.key === key ? { ...item, ...patch } : item)),
    );
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <Label>{label}</Label>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onChange([...items, itemDraft()])}
        >
          <Plus />
          Add item
        </Button>
      </div>
      {items.length === 0 && (
        <p className="text-muted-foreground text-sm">No items on this side.</p>
      )}
      {items.map((item) => (
        <div key={item.key} className="flex items-center gap-2">
          <Input
            list={ITEM_NAMES_ID}
            value={item.name}
            placeholder="Item name"
            aria-label={`${label} item name`}
            onChange={(e) => update(item.key, { name: e.target.value })}
          />
          <Input
            className="w-20"
            inputMode="numeric"
            value={item.count}
            aria-label={`${label} item count`}
            onChange={(e) => update(item.key, { count: e.target.value })}
          />
          <Input
            className="w-20"
            inputMode="numeric"
            value={item.rank}
            placeholder="Rank"
            aria-label={`${label} item rank`}
            onChange={(e) => update(item.key, { rank: e.target.value })}
          />
          <Button
            variant="ghost"
            size="icon"
            className="size-8"
            aria-label="Remove item"
            onClick={() =>
              onChange(items.filter((other) => other.key !== item.key))
            }
          >
            <X className="size-4" />
          </Button>
        </div>
      ))}
    </div>
  );
}

export function TradeDialog({
  stored,
  onSubmit,
  onClose,
}: {
  stored: StoredTrade | null;
  onSubmit: (entry: TradeEntry) => Promise<void>;
  onClose: () => void;
}) {
  const [draft, setDraft] = useState(() => draftOf(stored));
  const [saving, setSaving] = useState(false);
  const [itemNames, setItemNames] = useState<string[]>([]);

  useEffect(() => {
    api
      .marketItems()
      .then((items) => setItemNames(items.map((item) => item.name)))
      .catch((error) => logError("Loading item names failed", error));
  }, []);

  const entry = entryOf(draft);

  const submit = async () => {
    if (!entry) {
      return;
    }
    setSaving(true);
    await onSubmit(entry);
    setSaving(false);
    onClose();
  };

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>{stored ? "Edit Trade" : "Record Trade"}</DialogTitle>
          <DialogDescription>
            Offered is what you gave, received is what you got. A sale is
            platinum received with nothing on the received side, a purchase is
            platinum paid with nothing on the offered side.
          </DialogDescription>
        </DialogHeader>
        <datalist id={ITEM_NAMES_ID}>
          {itemNames.map((name) => (
            <option key={name} value={name} />
          ))}
        </datalist>
        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-2">
            <Label htmlFor="trade-at">When</Label>
            <DateTimePicker
              id="trade-at"
              value={draft.at}
              onChange={(at) => setDraft({ ...draft, at })}
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="trade-partner">Partner</Label>
            <Input
              id="trade-partner"
              value={draft.partner}
              placeholder="Player name"
              onChange={(e) => setDraft({ ...draft, partner: e.target.value })}
            />
          </div>
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="flex flex-col gap-2">
              <Label htmlFor="trade-plat">Platinum</Label>
              <Input
                id="trade-plat"
                inputMode="numeric"
                value={draft.plat}
                onChange={(e) => setDraft({ ...draft, plat: e.target.value })}
              />
            </div>
            <div className="flex flex-col gap-2">
              <Label htmlFor="trade-direction">Direction</Label>
              <Select
                value={draft.direction}
                onValueChange={(direction) =>
                  setDraft({
                    ...draft,
                    direction: direction as TradeDraft["direction"],
                  })
                }
              >
                <SelectTrigger id="trade-direction" className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="received">Received</SelectItem>
                  <SelectItem value="paid">Paid</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
          <ItemRows
            label="Offered"
            items={draft.offered}
            onChange={(offered) => setDraft({ ...draft, offered })}
          />
          <ItemRows
            label="Received"
            items={draft.received}
            onChange={(received) => setDraft({ ...draft, received })}
          />
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={submit} disabled={saving || !entry}>
            {stored ? "Save Trade" : "Record Trade"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
