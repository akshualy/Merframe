import { LogOut, Mail } from "lucide-react";
import {
  type ReactNode,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "react";
import { useSearchParams } from "react-router";
import { toast } from "sonner";
import { GameIcon } from "@/components/game-icon";
import { ErrorNote, Page, Quoted, Section } from "@/components/page";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useListen } from "@/hooks/use-listen";
import { api, errorMessage, events, logError, reportError } from "@/lib/bridge";
import { ago, MARKET_CHATS_URL, num } from "@/lib/format";
import { usePageQuote } from "@/lib/quotes";
import { cn } from "@/lib/utils";
import { useAppStore } from "@/stores/app-store";
import type {
  Auction,
  MarketItem,
  MarketSnapshot,
  OrderRow,
  OrderType,
} from "@/types";
import { AuctionsTable } from "./auctions";
import { BuySellPanel, type CompareRequest } from "./buy-sell";
import { LoginCard } from "./login-card";
import { OrdersTable } from "./orders";
import { PresenceControl } from "./presence";

type MarketTab = "orders" | "auctions";

export function MarketPage() {
  const quote = usePageQuote("market");
  const { status, setStatus } = useAppStore();
  const [orders, setOrders] = useState<OrderRow[]>([]);
  const [auctions, setAuctions] = useState<Auction[]>([]);
  const [items, setItems] = useState<MarketItem[]>([]);
  const [compare, setCompare] = useState<CompareRequest | null>(null);
  const [params, setParams] = useSearchParams();
  const tab = (params.get("tab") as MarketTab | null) ?? "orders";
  const [refreshedAt, setRefreshedAt] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [signedOut, setSignedOut] = useState(false);
  const account = status?.market_account ?? null;

  useEffect(() => {
    async function load() {
      try {
        setItems(await api.marketItems());
      } catch (error) {
        setItems([]);
        reportError(error);
      }
    }
    load();
  }, []);

  useEffect(() => {
    let last = 0;
    async function markActivity() {
      if (Date.now() - last < 5_000) {
        return;
      }
      last = Date.now();
      try {
        await api.marketActivity();
      } catch (error) {
        logError("Marking market activity", error);
      }
    }
    markActivity();
    document.addEventListener("click", markActivity);
    document.addEventListener("keydown", markActivity);
    return () => {
      document.removeEventListener("click", markActivity);
      document.removeEventListener("keydown", markActivity);
    };
  }, []);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const [nextOrders, nextAuctions] = await Promise.all([
        api.marketMyOrders(),
        api.marketMyAuctions(),
      ]);
      setOrders(nextOrders);
      setAuctions(nextAuctions);
      setRefreshedAt(new Date().toISOString());
      setError(null);
    } catch (error) {
      const message = errorMessage(error);
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (account) {
      reload();
    } else {
      setLoading(false);
    }
  }, [account, reload]);

  useListen(events.marketSignedOut, () => setSignedOut(true));

  useListen<MarketSnapshot>(events.marketUpdated, (snapshot) => {
    if (snapshot.orders) {
      setOrders(snapshot.orders);
    }
    if (snapshot.auctions) {
      setAuctions(snapshot.auctions);
    }
    setRefreshedAt(snapshot.at);
    setError(null);
  });

  const runMarketAction = useCallback(
    async (promise: Promise<unknown>, message: string) => {
      try {
        await promise;
      } catch (error) {
        reportError(error);
        return;
      }
      toast.success(message);
      reload();
    },
    [reload],
  );

  const compareSlug = useCallback(
    (slug: string, side: OrderType = "sell", rank: number | null = null) => {
      const item = items.find((candidate) => candidate.slug === slug);
      if (item) {
        setCompare({ item, side, rank, nonce: Date.now() });
      } else {
        toast.error("warframe.market does not list this item any more");
      }
    },
    [items],
  );

  const requestedItem = params.get("item");
  useEffect(() => {
    if (!requestedItem || items.length === 0) {
      return;
    }
    const rank = Number.parseInt(params.get("rank") ?? "", 10);
    compareSlug(
      requestedItem,
      params.get("side") === "buy" ? "buy" : "sell",
      Number.isFinite(rank) ? rank : null,
    );
    setParams(
      (next) => {
        next.delete("item");
        next.delete("side");
        next.delete("rank");
        return next;
      },
      { replace: true },
    );
  }, [requestedItem, items, params, compareSlug, setParams]);

  const handleSignedIn = useCallback(async () => {
    setSignedOut(false);
    try {
      setStatus(await api.gameStatus());
    } catch (error) {
      reportError(error);
    }
  }, [setStatus]);

  const setOrdersVisibility = useCallback(
    (visible: boolean) =>
      runMarketAction(
        api.marketSetVisibility(visible),
        visible ? "All orders visible" : "All orders hidden",
      ),
    [runMarketAction],
  );

  const setAuctionsVisibility = useCallback(
    (visible: boolean) =>
      runMarketAction(
        api.marketSetAuctionsVisibility(visible),
        visible ? "All auctions visible" : "All auctions hidden",
      ),
    [runMarketAction],
  );

  const handleOpenMessages = useCallback(async () => {
    try {
      await api.openUrl(MARKET_CHATS_URL);
    } catch (error) {
      reportError(error);
    }
  }, []);

  const handleSignOut = useCallback(async () => {
    try {
      await api.marketLogout();
      setStatus(await api.gameStatus());
    } catch (error) {
      reportError(error);
    }
  }, [setStatus]);

  const totals = useMemo(() => {
    const sell = orders.filter((order) => order.order_type === "sell");
    return {
      sell: sell.length,
      buy: orders.length - sell.length,
      plat: sell.reduce(
        (total, order) => total + order.platinum * order.quantity,
        0,
      ),
      missing: orders.filter((order) => order.show_warning).length,
    };
  }, [orders]);

  if (!account) {
    return (
      <Page title="warframe.market" description={<Quoted quote={quote} />}>
        {signedOut && (
          <ErrorNote message="warframe.market rejected the stored session" />
        )}
        <LoginCard onDone={handleSignedIn} />
        <BuySellPanel items={items} compare={compare} />
      </Page>
    );
  }

  const openAuctions = auctions.filter((auction) => !auction.closed).length;
  const unread = status?.market_unread ?? 0;

  return (
    <Page
      title="warframe.market"
      description={`Signed in as ${account.ingame_name}`}
      actions={
        <>
          <PresenceControl />
          <Button variant="outline" onClick={handleOpenMessages}>
            <Mail className="size-4" />
            Messages
            {unread > 0 && <Badge variant="accent">{num(unread)}</Badge>}
          </Button>
          <Button variant="outline" onClick={handleSignOut}>
            <LogOut className="size-4" />
            Sign Out
          </Button>
        </>
      }
    >
      {error && <ErrorNote message={error} />}

      <Section
        title="My Listings"
        description={refreshedAt ? `Refreshed ${ago(refreshedAt)}` : undefined}
        action={
          <dl className="flex flex-wrap items-center gap-x-5 gap-y-1 text-sm">
            <Figure label="Sell orders" value={num(totals.sell)} />
            <Figure label="Buy orders" value={num(totals.buy)} />
            <Figure label="Open auctions" value={num(openAuctions)} />
            <Figure
              label="Listed value"
              value={
                <span className="flex items-center gap-1">
                  {num(totals.plat)}
                  <GameIcon name="platinum" size={16} alt="Platinum" />
                </span>
              }
            />
            <Figure
              label="Missing items"
              value={num(totals.missing)}
              tone={totals.missing > 0 ? "text-warning" : undefined}
              title={
                totals.missing > 0
                  ? "Sell orders above what the account holds"
                  : undefined
              }
            />
          </dl>
        }
      >
        {loading && orders.length === 0 && auctions.length === 0 ? (
          <Skeleton className="h-48 w-full" />
        ) : (
          <div className={cn("transition-opacity", loading && "opacity-60")}>
            <Tabs
              value={tab}
              onValueChange={(next) =>
                setParams({ tab: next }, { replace: true })
              }
            >
              <TabsList className="mb-4">
                <TabsTrigger value="orders">
                  Orders ({num(orders.length)})
                </TabsTrigger>
                <TabsTrigger value="auctions">
                  Auctions ({num(auctions.length)})
                </TabsTrigger>
              </TabsList>
              <TabsContent value="orders">
                <OrdersTable
                  rows={orders}
                  onCompare={compareSlug}
                  onRefresh={reload}
                  onSetVisibility={setOrdersVisibility}
                  runMarketAction={runMarketAction}
                />
              </TabsContent>
              <TabsContent value="auctions">
                <AuctionsTable
                  auctions={auctions}
                  items={items}
                  onRefresh={reload}
                  onSetVisibility={setAuctionsVisibility}
                  runMarketAction={runMarketAction}
                />
              </TabsContent>
            </Tabs>
          </div>
        )}
      </Section>

      <BuySellPanel items={items} compare={compare} />
    </Page>
  );
}

function Figure({
  label,
  value,
  tone,
  title,
}: {
  label: string;
  value: ReactNode;
  tone?: string;
  title?: string;
}) {
  return (
    <div className="flex items-baseline gap-1.5" title={title}>
      <dt className="text-muted-foreground text-xs">{label}</dt>
      <dd className={cn("font-semibold tabular-nums", tone)}>{value}</dd>
    </div>
  );
}
