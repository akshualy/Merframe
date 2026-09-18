import { ArrowDownRight, ArrowUpRight, Search } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { GameIcon } from "@/components/game-icon";
import { MarketThumb } from "@/components/market-thumb";
import { EmptyNote, Quoted, Section } from "@/components/page";
import { Badge } from "@/components/ui/badge";
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
import { Skeleton } from "@/components/ui/skeleton";
import { api, reportError } from "@/lib/bridge";
import { marketUrl } from "@/lib/format";
import { usePageQuote } from "@/lib/quotes";
import { cn } from "@/lib/utils";
import type { MarketItem, Order, OrderBook, OrderType } from "@/types";

export interface CompareRequest {
  item: MarketItem;
  nonce: number;
}

function perTrade(order: Order) {
  return order.perTrade && order.perTrade > 1 ? order.perTrade : 1;
}

function unitPrice(order: Order) {
  return Math.round((10 * order.platinum) / perTrade(order)) / 10;
}

function orderDetail(order: Order) {
  if (order.rank !== undefined && order.rank !== null) {
    return `rank ${order.rank}`;
  }
  if (order.amberStars !== undefined || order.cyanStars !== undefined) {
    return `${order.amberStars ?? 0} amber, ${order.cyanStars ?? 0} cyan`;
  }
  return order.subtype ?? null;
}

function whisper(item: MarketItem, order: Order, wanted: OrderType) {
  const detail = orderDetail(order);
  const name = detail ? `${item.name} (${detail})` : item.name;
  const bundle = perTrade(order) > 1 ? ` x${perTrade(order)}` : "";
  const player = order.user?.ingameName ?? "";
  const verb = wanted === "buy" ? "buy" : "sell";
  return `/w ${player} Hi! I want to ${verb}:${bundle} "${name}" for ${order.platinum} platinum. (warframe.market)`;
}

function OrderList({
  item,
  orders,
  loading,
  side,
  best,
}: {
  item: MarketItem;
  orders: Order[];
  loading: boolean;
  side: OrderType;
  best: number | null;
}) {
  const [copied, setCopied] = useState<string | null>(null);
  const clearCopied = useRef(0);

  useEffect(() => () => window.clearTimeout(clearCopied.current), []);

  const handleCopy = async (order: Order) => {
    const wanted = side === "sell" ? "buy" : "sell";
    try {
      await navigator.clipboard.writeText(whisper(item, order, wanted));
    } catch (error) {
      reportError(error);
      return;
    }
    setCopied(order.id);
    window.clearTimeout(clearCopied.current);
    clearCopied.current = window.setTimeout(() => setCopied(null), 1500);
  };

  if (loading) {
    return <Skeleton className="h-32 w-full" />;
  }

  return (
    <ul className="flex flex-col gap-1">
      {orders.length === 0 && (
        <li className="text-muted-foreground text-sm">
          No trader is in game with this order.
        </li>
      )}
      {orders.slice(0, 5).map((order) => {
        const good =
          side === "buy" && best !== null && order.platinum >= best
            ? "border-accent"
            : undefined;
        const bundle = perTrade(order);
        const detail = orderDetail(order);
        return (
          <li key={order.id}>
            <button
              type="button"
              onClick={() => handleCopy(order)}
              className={cn(
                "hover:bg-secondary flex w-full cursor-pointer items-center gap-2 rounded-md border px-2 py-1 text-left text-sm",
                good,
              )}
            >
              {copied === order.id ? (
                <span className="text-muted-foreground flex-1">Copied!</span>
              ) : (
                <>
                  <span className="flex-1 truncate">
                    {order.user?.ingameName ?? ""}
                  </span>
                  <Badge variant="muted">x{order.quantity}</Badge>
                  {bundle > 1 && (
                    <Badge variant="secondary">{bundle} per trade</Badge>
                  )}
                  {detail && <Badge variant="muted">{detail}</Badge>}
                  <span
                    className={cn(
                      "flex w-24 items-center justify-end gap-0.5 font-bold",
                      side === "sell" ? "text-primary" : "text-accent",
                    )}
                  >
                    {order.platinum}
                    <GameIcon name="platinum" size={16} alt="Platinum" />
                    {bundle > 1 ? ` (${unitPrice(order)})` : ""}
                  </span>
                </>
              )}
            </button>
          </li>
        );
      })}
    </ul>
  );
}

export function BuySellPanel({
  items,
  compare,
}: {
  items: MarketItem[];
  compare: CompareRequest | null;
}) {
  const quote = usePageQuote("marketBook");
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const searchRef = useRef<HTMLDivElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const [selected, setSelected] = useState<MarketItem | null>(null);
  const [book, setBook] = useState<OrderBook | null>(null);
  const [loadingOrders, setLoadingOrders] = useState(false);
  const [platinum, setPlatinum] = useState("");
  const [quantity, setQuantity] = useState("1");
  const [rank, setRank] = useState("0");
  const [subtype, setSubtype] = useState("");
  const [amberStars, setAmberStars] = useState("0");
  const [cyanStars, setCyanStars] = useState("0");
  const [side, setSide] = useState<OrderType>("sell");
  const [posting, setPosting] = useState(false);

  const matches = useMemo(() => {
    const search = query.trim().toLowerCase();
    if (!search) {
      return [];
    }
    return items
      .filter((item) => item.name.toLowerCase().includes(search))
      .slice(0, 12);
  }, [items, query]);

  const isMod = selected?.max_rank != null;
  const subtypes = selected?.subtypes ?? [];
  const hasSubtypes = subtypes.length > 0;
  const hasAmberStars = selected?.max_amber_stars != null;
  const hasCyanStars = selected?.max_cyan_stars != null;

  const handlePick = useCallback(async (item: MarketItem) => {
    setSelected(item);
    setQuery(item.name);
    setOpen(false);
    setLoadingOrders(true);
    setRank("0");
    setSubtype(item.subtypes?.[0] ?? "");
    setAmberStars("0");
    setCyanStars("0");
    try {
      const next = await api.marketItemOrders(item.slug);
      setBook(next);
      const lowest = next.sell[0]?.platinum;
      setPlatinum(lowest === undefined ? "" : String(Math.max(1, lowest)));
    } catch (error) {
      setBook(null);
      reportError(error);
    } finally {
      setLoadingOrders(false);
    }
  }, []);

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

  useEffect(() => {
    if (!compare) {
      return;
    }
    handlePick(compare.item);
    panelRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
  }, [compare, handlePick]);

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

  const handlePost = useCallback(async () => {
    if (!selected) {
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
    setPosting(true);
    try {
      await api.marketPostOrder({
        item_id: selected.id,
        order_type: side,
        platinum: plat,
        quantity: Number.isFinite(count) ? count : 1,
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
    } catch (error) {
      reportError(error);
    } finally {
      setPosting(false);
    }
  }, [
    selected,
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

  const lowestSell = book?.sell[0]?.platinum ?? null;

  return (
    <div ref={panelRef} className="scroll-mt-6">
      <Section title="Buy / Sell Window" description={<Quoted quote={quote} />}>
        <div className="flex flex-col gap-4">
          <div ref={searchRef} className="relative max-w-md">
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
              <ul className="bg-popover absolute top-full z-20 mt-1 w-full overflow-hidden rounded-md border shadow-md">
                {matches.map((item) => (
                  <li key={item.id}>
                    <button
                      type="button"
                      onClick={() => handlePick(item)}
                      className="hover:bg-secondary flex w-full cursor-pointer items-center gap-2 px-3 py-1.5 text-left text-sm"
                    >
                      <MarketThumb thumb={item.thumb} name={item.name} />
                      <span className="truncate">{item.name}</span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>

          {selected ? (
            <div className="grid gap-4 lg:grid-cols-[1fr_1fr_auto]">
              <div className="flex flex-col gap-2">
                <span className="text-muted-foreground flex items-center gap-1 text-xs">
                  <ArrowUpRight className="size-3.5" /> Lowest sellers in game
                </span>
                <OrderList
                  item={selected}
                  orders={book?.sell ?? []}
                  loading={loadingOrders}
                  side="sell"
                  best={null}
                />
              </div>
              <div className="flex flex-col gap-2">
                <span className="text-muted-foreground flex items-center gap-1 text-xs">
                  <ArrowDownRight className="size-3.5" /> Highest buyers in game
                </span>
                <OrderList
                  item={selected}
                  orders={book?.buy ?? []}
                  loading={loadingOrders}
                  side="buy"
                  best={lowestSell}
                />
              </div>
              <div className="flex w-56 flex-col gap-2">
                <Hint as="span">Post an order</Hint>
                <Select
                  value={side}
                  onValueChange={(value) => setSide(value as OrderType)}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="sell">Sell</SelectItem>
                    <SelectItem value="buy">Buy</SelectItem>
                  </SelectContent>
                </Select>
                <div className="flex flex-col gap-1">
                  <Label htmlFor="order-platinum">Platinum</Label>
                  <Input
                    id="order-platinum"
                    value={platinum}
                    onChange={(e) => setPlatinum(e.target.value)}
                    inputMode="numeric"
                  />
                </div>
                <div className="flex flex-col gap-1">
                  <Label htmlFor="order-quantity">Quantity</Label>
                  <Input
                    id="order-quantity"
                    value={quantity}
                    onChange={(e) => setQuantity(e.target.value)}
                    inputMode="numeric"
                  />
                </div>
                {isMod && (
                  <div className="flex flex-col gap-1">
                    <Label htmlFor="order-rank">
                      Rank (max {selected.max_rank})
                    </Label>
                    <Input
                      id="order-rank"
                      value={rank}
                      onChange={(e) => setRank(e.target.value)}
                      inputMode="numeric"
                    />
                  </div>
                )}
                {(hasAmberStars || hasCyanStars) && (
                  <div className="flex gap-2">
                    {hasAmberStars && (
                      <div className="flex flex-1 flex-col gap-1">
                        <Label htmlFor="order-amber-stars">Amber stars</Label>
                        <Input
                          id="order-amber-stars"
                          value={amberStars}
                          onChange={(e) => setAmberStars(e.target.value)}
                          inputMode="numeric"
                        />
                      </div>
                    )}
                    {hasCyanStars && (
                      <div className="flex flex-1 flex-col gap-1">
                        <Label htmlFor="order-cyan-stars">Cyan stars</Label>
                        <Input
                          id="order-cyan-stars"
                          value={cyanStars}
                          onChange={(e) => setCyanStars(e.target.value)}
                          inputMode="numeric"
                        />
                      </div>
                    )}
                  </div>
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
                            {option}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                )}
                <Button onClick={handlePost} disabled={posting}>
                  Post {side === "sell" ? "Sell" : "Buy"} Order
                </Button>
                <Button variant="ghost" size="sm" onClick={handleOpenMarket}>
                  Open on warframe.market
                </Button>
              </div>
            </div>
          ) : (
            <EmptyNote>Pick an item to see the order book.</EmptyNote>
          )}
        </div>
      </Section>
    </div>
  );
}
