import { ChevronDown } from "lucide-react";
import { type ReactNode, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { FavouriteStar } from "@/components/favourite-star";
import { ItemImage, prefetchImages } from "@/components/item-image";
import {
  CardGrid,
  EmptyNote,
  ErrorNote,
  Page,
  Quoted,
  Section,
  Stat,
  Surface,
  TableSkeleton,
} from "@/components/page";
import { Badge } from "@/components/ui/badge";
import { CheckboxField } from "@/components/ui/checkbox";
import { Hint } from "@/components/ui/hint";
import { Progress } from "@/components/ui/progress";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useAsyncData } from "@/hooks/use-async-data";
import { useListen } from "@/hooks/use-listen";
import { api, errorMessage, events } from "@/lib/bridge";
import { num, percent } from "@/lib/format";
import { occurrenceKeys } from "@/lib/keys";
import { usePageQuote } from "@/lib/quotes";
import { cn } from "@/lib/utils";
import { useAppStore } from "@/stores/app-store";
import type {
  CategoryTotals,
  LevelUpRoute,
  MasteryOrdering,
  Settings,
} from "@/types";

const ORDERINGS: { value: MasteryOrdering; label: string }[] = [
  { value: "closest", label: "Closest to done" },
  { value: "from_relics", label: "Relics you own" },
  { value: "by_platinum", label: "Cheapest to buy" },
];

const INTRINSIC_KINDS = ["duviriIntrinsics", "railjackIntrinsics"];

function foundersNote(founder: boolean, included: boolean): string {
  if (included) {
    return "Excalibur Prime, Skana Prime and Lato Prime count towards mastery.";
  }
  return founder
    ? "The three Founders items are left out."
    : "The three Founders items are left out because this account has no Founder accolade.";
}

function RankBar({
  percent,
  favouritePercent,
  favouriteXp,
}: {
  percent: number;
  favouritePercent: number;
  favouriteXp: number;
}) {
  return (
    <span className="relative mt-1 block">
      <Progress value={percent} />
      {favouritePercent > 0 && (
        <span
          title={`Your favourites are worth ${num(favouriteXp)} XP of this rank`}
          className="bg-accent absolute inset-y-0 block"
          style={{ left: `${percent}%`, width: `${favouritePercent}%` }}
        />
      )}
    </span>
  );
}

function Line({ label, totals }: { label: string; totals?: CategoryTotals }) {
  return (
    <div className="grid grid-cols-[1fr_auto_auto] items-center gap-3 text-sm">
      <span className="text-muted-foreground">{label}</span>
      <span>{percent((totals?.percent ?? 0) / 100, 0)}</span>
      <span className="text-foreground w-20 text-right font-medium">
        {num(totals?.current)}/{num(totals?.max)}
      </span>
    </div>
  );
}

function Tile({
  title,
  progress,
  children,
}: {
  title: string;
  progress: number;
  children: ReactNode;
}) {
  return (
    <Surface className="gap-3">
      <div className="flex items-baseline justify-between">
        <span className="font-semibold">{title}</span>
        <span className="text-primary text-lg font-bold">
          {percent(progress / 100, 0)}
        </span>
      </div>
      <Progress value={progress} />
      <div className="flex flex-col gap-1">{children}</div>
    </Surface>
  );
}

function RouteCard({
  route,
  open,
  onToggle,
}: {
  route: LevelUpRoute;
  open: boolean;
  onToggle: () => void;
}) {
  const memberKeys = occurrenceKeys(
    route.members.map((member) => `${member.name}:${member.detail}`),
  );
  return (
    <div className="bg-card flex flex-col rounded-xl border">
      <button
        type="button"
        onClick={onToggle}
        className="grid cursor-pointer grid-cols-[auto_1fr_auto] items-center gap-3 px-3 py-2 text-left"
      >
        <ChevronDown
          className={cn(
            "text-muted-foreground size-3.5 shrink-0 transition-transform",
            !open && "-rotate-90",
          )}
        />
        <span className="flex min-w-0 flex-col">
          <span className="truncate text-sm font-medium">{route.label}</span>
          <Hint as="span">
            {num(route.count)}{" "}
            {route.count === 1 ? route.unit : `${route.unit}s`}
          </Hint>
        </span>
        <span className="text-primary text-sm font-bold">
          +{num(route.xp_available)} XP
        </span>
      </button>
      {open && (
        <ul className="max-h-64 overflow-y-auto border-t px-3 py-2">
          {route.members.map((member, index) => (
            <li
              key={memberKeys[index]}
              className="grid grid-cols-[1fr_auto] items-baseline gap-3 py-0.5 text-xs"
            >
              <span className="min-w-0 truncate">
                {member.name}
                {member.detail && (
                  <span className="text-muted-foreground">
                    {" "}
                    {member.detail}
                  </span>
                )}
              </span>
              <span className="text-muted-foreground tabular-nums">
                +{num(member.xp)} XP
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export function MasteryPage() {
  const quote = usePageQuote("mastery");
  const { settings, setSettings } = useAppStore();
  const [ordering, setOrdering] = useState<MasteryOrdering>("closest");
  const [saveError, setSaveError] = useState<string | null>(null);
  const [openRoute, setOpenRoute] = useState<string | null>(null);

  const settingsRead = settings !== null;
  const includeForma = settings?.include_forma_ranks ?? true;
  const founders = settings?.include_founders_items ?? null;

  const load = useMemo(
    () =>
      settingsRead
        ? () => api.masteryTab(ordering, founders, includeForma)
        : null,
    [settingsRead, ordering, founders, includeForma],
  );

  const { data, error, loading, reload } = useAsyncData(load);

  useListen(events.inventoryUpdated, reload);
  useListen(events.pricesUpdated, reload);
  useListen(events.appReady, reload);

  const topItems = data?.recommended ?? [];
  const routes = data?.routes ?? [];
  const routeTotal = routes.reduce((sum, route) => sum + route.xp_available, 0);

  useEffect(() => {
    prefetchImages((data?.recommended ?? []).map((item) => item.image_name));
  }, [data?.recommended]);

  const choose = async (patch: Partial<Settings>) => {
    if (!settings) {
      return;
    }
    const next = { ...settings, ...patch };
    setSettings(next);
    setSaveError(null);
    try {
      await api.settingsSet(next);
    } catch (error) {
      const message = errorMessage(error);
      setSaveError(message);
      toast.error(message);
    }
  };

  const founder = data?.founder ?? false;
  const includeFounders = founders ?? data?.include_founders ?? false;

  const foundersToggle = (
    <CheckboxField
      id="includeFounders"
      checked={includeFounders}
      onChange={(value) => choose({ include_founders_items: value })}
    >
      Founders items
    </CheckboxField>
  );

  if (loading && !data) {
    return (
      <Page title="Mastery" description={<Quoted quote={quote} />}>
        <TableSkeleton />
      </Page>
    );
  }

  const summary = data?.summary;
  const problem = error ?? saveError;

  return (
    <Page
      title="Mastery"
      description={<Quoted quote={quote} />}
      actions={foundersToggle}
    >
      {problem && <ErrorNote message={problem} />}

      <div className="grid gap-4 sm:grid-cols-3">
        <Stat label="Mastery rank" value={num(data?.rank)} />
        <Stat
          label="Into this rank"
          value={`${num(data?.rank_xp_earned)} / ${num(data?.rank_xp_span)} XP`}
        />
        <Stat
          label="Rank progress"
          value={percent((data?.percent ?? 0) / 100, 0)}
          hint={
            <RankBar
              percent={data?.percent ?? 0}
              favouritePercent={data?.favourite_percent ?? 0}
              favouriteXp={data?.favourite_xp ?? 0}
            />
          }
        />
      </div>

      <div className="grid gap-4 lg:grid-cols-3">
        <Tile title="Game content" progress={summary?.content_percent ?? 0}>
          <Line label="Warframes / Archwings" totals={summary?.warframes} />
          <Line label="Weapons" totals={summary?.weapons} />
          <Line label="Companions" totals={summary?.companions} />
        </Tile>
        <Tile title="Star chart" progress={summary?.star_percent ?? 0}>
          <Line label="Normal" totals={summary?.star_normal} />
          <Line label="Junctions" totals={summary?.star_junctions} />
          <Line label="Steel Path" totals={summary?.star_steel} />
          <Line
            label="Steel Path junctions"
            totals={summary?.star_steel_junctions}
          />
        </Tile>
        <Tile title="Intrinsics" progress={summary?.intrinsic_percent ?? 0}>
          <Line label="Railjack" totals={summary?.intrinsic_railjack} />
          <Line label="Duviri" totals={summary?.intrinsic_duviri} />
        </Tile>
      </div>

      <Section title="Mastery Left to Earn">
        {routes.length === 0 ? (
          <EmptyNote>Everything is mastered.</EmptyNote>
        ) : (
          <div
            className={cn(
              "grid items-start gap-2 transition-opacity sm:grid-cols-2 xl:grid-cols-3",
              loading && "opacity-60",
            )}
          >
            {routes.map((route) => (
              <RouteCard
                key={route.kind}
                route={route}
                open={openRoute === route.kind}
                onToggle={() =>
                  setOpenRoute(openRoute === route.kind ? null : route.kind)
                }
              />
            ))}
          </div>
        )}
        {routes.length > 0 && (
          <p className="text-muted-foreground mt-3 text-right text-sm">
            Total{" "}
            <span className="text-primary font-bold">{num(routeTotal)}</span> XP
          </p>
        )}
      </Section>

      <Section
        title="Fastest Mastery From Here"
        description={foundersNote(founder, includeFounders)}
        action={
          <div className="flex flex-wrap items-center gap-3">
            <CheckboxField
              id="includeForma"
              checked={includeForma}
              onChange={(value) => choose({ include_forma_ranks: value })}
            >
              Include ranks past 30
            </CheckboxField>
            <Tabs
              value={ordering}
              onValueChange={(value) => setOrdering(value as MasteryOrdering)}
            >
              <TabsList>
                {ORDERINGS.map((entry) => (
                  <TabsTrigger key={entry.value} value={entry.value}>
                    {entry.label}
                  </TabsTrigger>
                ))}
              </TabsList>
            </Tabs>
          </div>
        }
      >
        <CardGrid className={cn("transition-opacity", loading && "opacity-60")}>
          {topItems.length === 0 ? (
            <EmptyNote>Nothing matches these filters.</EmptyNote>
          ) : (
            topItems.map((item) => (
              <div
                key={item.unique_name}
                className="bg-card flex gap-3 rounded-xl border p-3"
              >
                <ItemImage
                  imageName={item.image_name}
                  size={52}
                  alt={item.name}
                />
                <div className="flex min-w-0 flex-1 flex-col gap-1">
                  <span className="flex items-center gap-1">
                    {!INTRINSIC_KINDS.includes(item.kind) && (
                      <FavouriteStar
                        uniqueName={item.unique_name}
                        favourite={item.favourite}
                        size="small"
                      />
                    )}
                    <span className="truncate text-sm font-medium">
                      {item.name}
                    </span>
                  </span>
                  <Hint as="span">+{num(item.level.xp_remaining)} XP</Hint>
                  <span className="flex flex-wrap items-center gap-1">
                    {item.owned && item.level.max > 0 && (
                      <Badge variant="secondary">
                        Level {item.level.current}/{item.level.max}
                      </Badge>
                    )}
                    {item.owned && <Badge variant="secondary">Owned</Badge>}
                    {!item.owned && (
                      <Badge variant="muted">
                        {percent(item.acquisition.relic_probability, 2)} relics
                      </Badge>
                    )}
                    {!item.owned && (
                      <Badge variant="accent">
                        {num(item.acquisition.plat_cost)} plat
                      </Badge>
                    )}
                  </span>
                  {!item.owned && item.components.length > 0 && (
                    <span className="flex flex-wrap items-center gap-1 pt-1">
                      {item.components.map((component) => (
                        <span
                          key={component.unique_name}
                          title={component.name}
                          className={
                            component.enough ? "" : "opacity-40 grayscale"
                          }
                        >
                          <ItemImage
                            imageName={component.image_name}
                            size={20}
                          />
                        </span>
                      ))}
                    </span>
                  )}
                </div>
              </div>
            ))
          )}
        </CardGrid>
        {ordering === "by_platinum" && (data?.plat_total ?? 0) > 0 && (
          <p className="text-muted-foreground mt-3 text-right text-sm">
            Total{" "}
            <span className="text-primary font-bold">
              {num(data?.plat_total)}
            </span>{" "}
            platinum
          </p>
        )}
      </Section>
    </Page>
  );
}
