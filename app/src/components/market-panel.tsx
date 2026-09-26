import { ExternalLink, Search, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { ItemImage } from "@/components/item-image";
import { OrderList } from "@/components/market-order-list";
import { EmptyNote } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { api, reportError } from "@/lib/bridge";
import { marketUrl } from "@/lib/format";
import { capitalize } from "@/lib/world";
import { useAppStore } from "@/stores/app-store";
import { useMarketPanelStore } from "@/stores/market-panel-store";
import type { ItemListings, MarketItem, OrderType } from "@/types";

function NumberField({
  id,
  label,
  value,
  onChange,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <div className="flex flex-col gap-1">
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        inputMode="numeric"
      />
    </div>
  );
}

function defaultSubtype(item: MarketItem) {
  const subtypes = item.subtypes ?? [];
  return subtypes.includes("revealed") ? "revealed" : (subtypes[0] ?? "");
}

function ItemSearch({
  items,
  onPick,
}: {
  items: MarketItem[];
  onPick: (item: MarketItem) => void;
}) {
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const searchRef = useRef<HTMLDivElement>(null);

  const matches = useMemo(() => {
    const search = query.trim().toLowerCase();
    if (!search) {
      return [];
    }
    return items
      .filter((item) => item.name.toLowerCase().includes(search))
      .sort(
        (a, b) =>
          Number(b.name.toLowerCase() === search) -
          Number(a.name.toLowerCase() === search),
      )
      .slice(0, 20);
  }, [items, query]);

  useEffect(() => {
    if (!open) {
      return;
    }
    const handleClickOutside = (e: MouseEvent) => {
      if (!searchRef.current?.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [open]);

  return (
    <div ref={searchRef} className="relative">
      <Search className="text-muted-foreground pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2" />
      <Input
        value={query}
        onChange={(e) => {
          setQuery(e.target.value);
          setOpen(true);
        }}
        onFocus={() => setOpen(query.trim().length > 0)}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            setOpen(false);
          }
        }}
        placeholder="Search warframe.market items"
        className="pl-9"
      />
      {open && matches.length > 0 && (
        <ul className="bg-popover absolute top-full z-20 mt-1 max-h-80 w-full overflow-y-auto rounded-md border shadow-md">
          {matches.map((item) => (
            <li key={item.id}>
              <button
                type="button"
                onClick={() => {
                  setQuery("");
                  setOpen(false);
                  onPick(item);
                }}
                className="hover:bg-secondary flex w-full cursor-pointer items-center gap-2 px-3 py-1.5 text-left text-sm"
              >
                <ItemImage
                  imageName={item.image_name}
                  size={24}
                  alt={item.name}
                />
                <span className="truncate">{item.name}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export function MarketPanel() {
  const { items, request, hide } = useMarketPanelStore();
  const signedIn = useAppStore((state) => state.status?.market_account) != null;
  const navigate = useNavigate();
  const [selected, setSelected] = useState<MarketItem | null>(null);
  const [listings, setListings] = useState<ItemListings | null>(null);
  const [loadingOrders, setLoadingOrders] = useState(false);
  const [platinum, setPlatinum] = useState("");
  const [quantity, setQuantity] = useState("1");
  const [rank, setRank] = useState("0");
  const [subtype, setSubtype] = useState("");
  const [amberStars, setAmberStars] = useState("0");
  const [cyanStars, setCyanStars] = useState("0");
  const [side, setSide] = useState<OrderType>("sell");
  const [posting, setPosting] = useState(false);

  const isMod = selected?.max_rank != null;
  const subtypes = selected?.subtypes ?? [];
  const hasSubtypes = subtypes.length > 0;
  const hasAmberStars = selected?.max_amber_stars != null;
  const hasCyanStars = selected?.max_cyan_stars != null;

  const handlePick = useCallback(
    async (
      item: MarketItem,
      side: OrderType = "sell",
      rank: number | null = null,
    ) => {
      setSelected(item);
      setLoadingOrders(true);
      setSide(side);
      setRank(String(rank ?? 0));
      setSubtype(defaultSubtype(item));
      setAmberStars("0");
      setCyanStars("0");
      try {
        const next = await api.marketItemOrders(item.slug);
        setListings(next);
        const lowest = next.sell[0]?.platinum;
        setPlatinum(lowest === undefined ? "" : String(Math.max(1, lowest)));
      } catch (error) {
        setListings(null);
        reportError(error);
      } finally {
        setLoadingOrders(false);
      }
    },
    [],
  );

  useEffect(() => {
    if (!request) {
      return;
    }
    const item = items.find((candidate) => candidate.slug === request.slug);
    if (item) {
      handlePick(item, request.side, request.rank);
    } else {
      toast.error("warframe.market does not list this item any more");
    }
  }, [request, items, handlePick]);

  const handleOpenMarket = useCallback(async () => {
    if (!selected) {
      return;
    }
    try {
      await api.openUrl(marketUrl(selected.slug));
    } catch (error) {
      reportError(error);
    }
  }, [selected]);

  const handlePost = useCallback(async () => {
    if (!selected) {
      return;
    }
    if (!signedIn) {
      navigate("/market");
      return;
    }
    const plat = Number.parseInt(platinum, 10);
    const count = Number.parseInt(quantity, 10);
    const modRank = Number.parseInt(rank, 10);
    const amber = Number.parseInt(amberStars, 10);
    const cyan = Number.parseInt(cyanStars, 10);
    if (!Number.isFinite(plat) || plat <= 0) {
      toast.error("A platinum price is required");
      return;
    }
    if (!Number.isFinite(count) || count <= 0) {
      toast.error("A quantity of at least 1 is required");
      return;
    }
    if (hasSubtypes && !subtype) {
      toast.error("A type is required for this item");
      return;
    }
    setPosting(true);
    try {
      await api.marketPostOrder({
        item_id: selected.id,
        order_type: side,
        platinum: plat,
        quantity: count,
        visible: true,
        per_trade: selected.bulk_tradable ? 1 : undefined,
        rank: isMod && Number.isFinite(modRank) ? modRank : undefined,
        subtype: hasSubtypes ? subtype : undefined,
        amber_stars:
          hasAmberStars && Number.isFinite(amber) ? amber : undefined,
        cyan_stars: hasCyanStars && Number.isFinite(cyan) ? cyan : undefined,
      });
      toast.success(
        `${side === "sell" ? "Sell" : "Buy"} order posted for ${selected.name}`,
      );
      setSelected(null);
      setListings(null);
      setQuantity("1");
    } catch (error) {
      reportError(error);
    } finally {
      setPosting(false);
    }
  }, [
    selected,
    signedIn,
    navigate,
    platinum,
    quantity,
    rank,
    amberStars,
    cyanStars,
    side,
    isMod,
    hasSubtypes,
    subtype,
    hasAmberStars,
    hasCyanStars,
  ]);

  const lowestSell = listings?.sell[0]?.platinum ?? null;

  return (
    <aside className="bg-card flex w-96 shrink-0 flex-col border-l">
      <div className="flex h-14 shrink-0 items-center justify-between border-b px-4">
        <span className="text-foreground font-bold">warframe.market</span>
        <Button variant="ghost" size="icon" onClick={hide} title="Close">
          <X className="size-4" />
        </Button>
      </div>
      <div className="flex min-h-0 flex-1 flex-col gap-3 p-4">
        <ItemSearch items={items} onPick={handlePick} />
        {selected ? (
          <>
            <div className="flex items-center gap-2">
              <ItemImage
                imageName={selected.image_name}
                size={32}
                alt={selected.name}
              />
              <span className="flex-1 truncate font-semibold">
                {selected.name}
              </span>
              <Button
                variant="ghost"
                size="icon"
                onClick={handleOpenMarket}
                title="Open on warframe.market"
              >
                <ExternalLink className="size-4" />
              </Button>
            </div>
            <Tabs
              value={side}
              onValueChange={(value) => setSide(value as OrderType)}
            >
              <TabsList className="w-full">
                <TabsTrigger value="sell">WTS</TabsTrigger>
                <TabsTrigger value="buy">WTB</TabsTrigger>
              </TabsList>
            </Tabs>
            <Hint as="span">
              {side === "sell"
                ? "Lowest sellers in game, click to copy a whisper"
                : "Highest buyers in game, click to copy a whisper"}
            </Hint>
            <div className="min-h-0 flex-1 overflow-y-auto">
              <OrderList
                item={selected}
                orders={
                  side === "sell"
                    ? (listings?.sell ?? [])
                    : (listings?.buy ?? [])
                }
                loading={loadingOrders}
                side={side}
                best={side === "buy" ? lowestSell : null}
              />
            </div>
          </>
        ) : (
          <EmptyNote>
            Pick an item here or from a Sell or Buy button to see its
            warframe.market listings.
          </EmptyNote>
        )}
      </div>
      {selected && (
        <div className="flex shrink-0 flex-col gap-2 border-t p-4">
          <div className="grid grid-cols-2 gap-2">
            <NumberField
              id="order-platinum"
              label="Platinum"
              value={platinum}
              onChange={setPlatinum}
            />
            <NumberField
              id="order-quantity"
              label="Quantity"
              value={quantity}
              onChange={setQuantity}
            />
            {isMod && (
              <NumberField
                id="order-rank"
                label={`Rank (max ${selected.max_rank})`}
                value={rank}
                onChange={setRank}
              />
            )}
            {hasAmberStars && (
              <NumberField
                id="order-amber-stars"
                label="Amber stars"
                value={amberStars}
                onChange={setAmberStars}
              />
            )}
            {hasCyanStars && (
              <NumberField
                id="order-cyan-stars"
                label="Cyan stars"
                value={cyanStars}
                onChange={setCyanStars}
              />
            )}
            {hasSubtypes && (
              <div className="flex flex-col gap-1">
                <Label htmlFor="order-subtype">Type</Label>
                <Select value={subtype} onValueChange={setSubtype}>
                  <SelectTrigger id="order-subtype">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {subtypes.map((option) => (
                      <SelectItem key={option} value={option}>
                        {capitalize(option)}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            )}
          </div>
          <Button onClick={handlePost} disabled={posting}>
            Post {side === "sell" ? "Sell" : "Buy"} Order
          </Button>
        </div>
      )}
    </aside>
  );
}
