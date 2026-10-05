import { useEffect, useRef } from "react";
import { CardGrid, EmptyNote, Page, Quoted, Section } from "@/components/page";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useNow } from "@/hooks/use-now";
import { api, reportError } from "@/lib/bridge";
import { secondsUntil } from "@/lib/format";
import { usePageQuote } from "@/lib/quotes";
import { cn } from "@/lib/utils";
import { useAppStore } from "@/stores/app-store";
import { usePreferencesStore } from "@/stores/preferences-store";
import type { Fissure, FissurePath } from "@/types";
import { FissuresByTier } from "./fissures";
import { MarketSalesPanel } from "./market-sales";
import {
  ArchonPanel,
  BaroPanel,
  CircuitPanel,
  DarvoPanel,
  NightwavePanel,
  ResetsPanel,
  ResurgencePanel,
  SortiePanel,
} from "./panels";
import { TimerChip } from "./timers";

const PATH_FILTERS: { value: FissurePath; label: string }[] = [
  { value: "normal", label: "Normal" },
  { value: "hard", label: "Steel Path" },
  { value: "storm", label: "Void Storms" },
  { value: "all", label: "All" },
];

export function WorldPage() {
  const quote = usePageQuote("world");
  const data = useAppStore((state) => state.world);
  const setWorld = useAppStore((state) => state.setWorld);
  const now = useNow();
  const { fissurePath, setFissurePath } = usePreferencesStore();

  const nextCycleEnd = data?.timers.length
    ? Math.min(...data.timers.map((timer) => new Date(timer.ends).getTime()))
    : null;
  const rolledOver = useRef<number | null>(null);

  useEffect(() => {
    if (
      nextCycleEnd === null ||
      now < nextCycleEnd ||
      rolledOver.current === nextCycleEnd
    ) {
      return;
    }
    rolledOver.current = nextCycleEnd;
    api.worldstate().then(setWorld, reportError);
  }, [now, nextCycleEnd, setWorld]);

  const shown: Record<FissurePath, (fissure: Fissure) => boolean> = {
    all: () => true,
    normal: (fissure) => !fissure.steel_path && !fissure.is_storm,
    hard: (fissure) => fissure.steel_path,
    storm: (fissure) => fissure.is_storm,
  };
  const fissures = (data?.fissures ?? [])
    .filter((fissure) => secondsUntil(fissure.expiry, now) > 0)
    .filter(shown[fissurePath])
    .sort(
      (left, right) =>
        new Date(left.expiry).getTime() - new Date(right.expiry).getTime(),
    );

  const sortie = data?.sortie ?? null;
  const baro = data?.baro ?? null;
  const baroPresent = baro !== null && "Present" in baro;
  const baroPanel = (
    <BaroPanel baro={baro} manifest={data?.baro_manifest ?? []} now={now} />
  );

  return (
    <Page title="World" description={<Quoted quote={quote} />}>
      {data?.timers.length ? (
        <CardGrid>
          {data.timers.map((timer) => (
            <TimerChip key={timer.name} timer={timer} now={now} />
          ))}
        </CardGrid>
      ) : (
        <EmptyNote>The world state has not been fetched yet.</EmptyNote>
      )}

      <Section
        title="Void Fissures"
        description={`${fissures.length} active`}
        action={
          <Tabs
            value={fissurePath}
            onValueChange={(value) => setFissurePath(value as FissurePath)}
          >
            <TabsList>
              {PATH_FILTERS.map(({ value, label }) => (
                <TabsTrigger key={value} value={value}>
                  {label}
                </TabsTrigger>
              ))}
            </TabsList>
          </Tabs>
        }
      >
        <FissuresByTier fissures={fissures} now={now} />
      </Section>

      <div
        className={cn(
          "grid items-start gap-4 @lg:grid-cols-2",
          baroPresent && "@5xl:grid-cols-3",
        )}
      >
        {baroPresent && baroPanel}
        <div className="flex flex-col gap-4">
          {!baroPresent && baroPanel}
          <ResetsPanel sortie={sortie} now={now} />
          <DarvoPanel deals={data?.daily_deals ?? []} now={now} />
          <CircuitPanel circuit={data?.circuit ?? null} now={now} />
          <SortiePanel sortie={sortie} now={now} />
          <ArchonPanel hunt={data?.archon_hunt ?? null} now={now} />
        </div>
        <div className="flex flex-col gap-4">
          <ResurgencePanel
            resurgence={data?.prime_resurgence ?? null}
            now={now}
          />
          <MarketSalesPanel sales={data?.market_sales ?? []} now={now} />
          <NightwavePanel nightwave={data?.nightwave ?? null} now={now} />
        </div>
      </div>
    </Page>
  );
}
