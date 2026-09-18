import { useEffect, useRef } from "react";
import {
  CardGrid,
  EmptyNote,
  ErrorNote,
  Page,
  Quoted,
  Section,
  TableSkeleton,
} from "@/components/page";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useCommand } from "@/hooks/use-command";
import { useListen } from "@/hooks/use-listen";
import { useNow } from "@/hooks/use-now";
import { events } from "@/lib/bridge";
import { secondsUntil } from "@/lib/format";
import { usePageQuote } from "@/lib/quotes";
import { usePreferencesStore } from "@/stores/preferences-store";
import type { FissurePath } from "@/types";
import { FissuresByTier } from "./fissures";
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
  { value: "all", label: "Both" },
];

export function WorldPage() {
  const quote = usePageQuote("world");
  const { data, error, loading, reload } = useCommand("worldstate");
  const now = useNow();
  const { fissurePath, setFissurePath } = usePreferencesStore();

  useListen(events.worldStateUpdated, reload);

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
    reload();
  }, [now, nextCycleEnd, reload]);

  const fissures = (data?.fissures ?? [])
    .filter((fissure) => secondsUntil(fissure.expiry, now) > 0)
    .filter(
      (fissure) =>
        fissurePath === "all" ||
        fissure.steel_path === (fissurePath === "hard"),
    )
    .sort(
      (left, right) =>
        new Date(left.expiry).getTime() - new Date(right.expiry).getTime(),
    );

  if (loading && !data) {
    return (
      <Page title="World" description={<Quoted quote={quote} />}>
        <TableSkeleton />
      </Page>
    );
  }

  const sortie = data?.sortie ?? null;

  return (
    <Page title="World" description={<Quoted quote={quote} />}>
      {error && <ErrorNote message={error} />}

      {data?.timers.length ? (
        <CardGrid>
          {data.timers.map((timer) => (
            <TimerChip key={timer.name} timer={timer} now={now} />
          ))}
        </CardGrid>
      ) : (
        <EmptyNote>The world state has not been fetched yet.</EmptyNote>
      )}

      <div className="grid gap-4 xl:grid-cols-[1fr_22rem]">
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

        <div className="flex flex-col gap-4">
          <BaroPanel
            baro={data?.baro ?? null}
            manifest={data?.baro_manifest ?? []}
            now={now}
          />
          <ResurgencePanel
            resurgence={data?.prime_resurgence ?? null}
            now={now}
          />
          <DarvoPanel deals={data?.daily_deals ?? []} now={now} />
          <ResetsPanel sortie={sortie} now={now} />
          <CircuitPanel circuit={data?.circuit ?? null} now={now} />
          <NightwavePanel nightwave={data?.nightwave ?? null} now={now} />
          <SortiePanel sortie={sortie} now={now} />
          <ArchonPanel hunt={data?.archon_hunt ?? null} now={now} />
        </div>
      </div>
    </Page>
  );
}
