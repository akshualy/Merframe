import { ChevronDown } from "lucide-react";
import { type ReactNode, useMemo, useState } from "react";
import { Amount } from "@/components/currency";
import { ItemImage } from "@/components/item-image";
import { Section, Surface } from "@/components/page";
import { Badge } from "@/components/ui/badge";
import { Hint } from "@/components/ui/hint";
import { Progress } from "@/components/ui/progress";
import { SwitchField } from "@/components/ui/switch";
import { matchesText, toggled } from "@/lib/filters";
import { counted, num } from "@/lib/format";
import { cn } from "@/lib/utils";
import type {
  SyndicateDailyStanding,
  SyndicateKind,
  SyndicateOverview,
  SyndicatePledgeRelation,
  SyndicateProgressMeter,
  SyndicateRankStanding,
  SyndicateRankUp,
  SyndicatesTab,
} from "@/types";
import { ResetChecklistCountdown } from "./reset-countdown";
import { SyndicateOffers, useSyndicateOffers } from "./syndicate-offers";
import { ResetChecklistTile } from "./tiles";

const PLEDGE_MARKS: Record<
  SyndicatePledgeRelation,
  { label: string; tone: string }
> = {
  pledged: { label: "Pledged", tone: "text-primary" },
  allied: { label: "Allied", tone: "text-accent" },
  opposed: { label: "Opposed", tone: "text-vaulted" },
  enemy: { label: "Enemy", tone: "text-warning" },
};

function SyndicatePledgeMark({
  relation,
}: {
  relation: SyndicatePledgeRelation;
}) {
  const { label, tone } = PLEDGE_MARKS[relation];

  return <span className={tone}>({label})</span>;
}

const KIND_SWITCHES: { kind: SyndicateKind; label: string }[] = [
  { kind: "event", label: "Exclude event syndicates" },
  { kind: "conclave", label: "Exclude Conclave" },
];

function rankText(rank: SyndicateRankStanding): string {
  const position = `Rank ${rank.level}/${rank.top_level}`;
  return rank.title === null ? position : `${position}, ${rank.title}`;
}

function rankUpStandingText(next: SyndicateRankUp): string {
  return next.standing_short === 0
    ? `Enough standing for ${next.title}`
    : `${num(next.standing_short)} standing to ${next.title}`;
}

function SyndicateRankUpRequirements({ next }: { next: SyndicateRankUp }) {
  const [open, setOpen] = useState(false);

  if (next.credits === 0 && next.items.length === 0) {
    return <Hint as="span">{rankUpStandingText(next)}</Hint>;
  }

  return (
    <div className="flex flex-col gap-1">
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
        className="text-muted-foreground hover:text-foreground flex cursor-pointer items-center gap-1 text-left text-xs"
      >
        <ChevronDown
          className={cn(
            "size-3.5 shrink-0 transition-transform",
            !open && "-rotate-90",
          )}
        />
        Rank-Up requirements
      </button>
      {open && (
        <ul className="text-muted-foreground flex flex-col gap-1 pl-4.5 text-xs">
          <li>{rankUpStandingText(next)}</li>
          {next.credits > 0 && (
            <li className="flex">
              <Amount currency="credits" plain>
                {num(next.credits)}
              </Amount>
            </li>
          )}
          {next.items.map((item) => (
            <li key={item.name} className="flex items-center gap-1">
              <ItemImage imageName={item.image_name} size={16} />
              {item.name} {num(item.owned)}/{num(item.needed)}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function SyndicateMeter({
  meter,
  standing,
}: {
  meter: SyndicateProgressMeter;
  standing: number | null;
}) {
  return (
    <>
      <span className="flex items-baseline justify-between gap-2">
        <span>
          {num(meter.earned)}/{num(meter.span)}
        </span>
        {standing !== null && standing !== meter.earned && (
          <Hint as="span">{num(standing)} total</Hint>
        )}
      </span>
      <Progress value={(meter.earned / meter.span) * 100} />
    </>
  );
}

function SyndicateTile({
  syndicate,
  matches,
}: {
  syndicate: SyndicateOverview;
  matches: number | null;
}) {
  const { standing, meter, rank, daily, pledge } = syndicate;
  return (
    <ResetChecklistTile className={cn(matches === 0 && "opacity-50")}>
      <span className="flex items-center gap-2">
        <span className="truncate font-medium">{syndicate.name}</span>
        {pledge && <SyndicatePledgeMark relation={pledge} />}
        {matches !== null && matches > 0 && (
          <Badge variant="accent" className="ml-auto">
            {counted(matches, "match", "matches")}
          </Badge>
        )}
      </span>
      <Hint as="span" className="h-4">
        {rank && rankText(rank)}
      </Hint>
      {meter && <SyndicateMeter meter={meter} standing={standing} />}
      {rank?.next && <SyndicateRankUpRequirements next={rank.next} />}
      {daily && <Hint as="span">{num(daily.left)} left today</Hint>}
    </ResetChecklistTile>
  );
}

function SyndicateTileGrid({ children }: { children: ReactNode }) {
  return (
    <div className="grid gap-3 @sm:grid-cols-2 @3xl:grid-cols-3">
      {children}
    </div>
  );
}

function SyndicateFactionStanding({
  daily,
  children,
}: {
  daily: SyndicateDailyStanding;
  children: ReactNode;
}) {
  return (
    <Surface className="gap-3 p-3">
      <div className="flex flex-col gap-1.5 text-sm">
        <span className="flex items-baseline justify-between gap-2">
          <span className="font-medium">Faction Syndicates</span>
          <Hint as="span">
            {num(daily.left)} left of {num(daily.cap)} today
          </Hint>
        </span>
        <Progress value={(daily.left / daily.cap) * 100} />
      </div>
      <SyndicateTileGrid>{children}</SyndicateTileGrid>
    </Surface>
  );
}

export function ResetChecklistSyndicates({
  tab,
  now,
}: {
  tab: SyndicatesTab;
  now: number;
}) {
  const { syndicates } = tab;
  const [search, setSearch] = useState("");
  const [excluded, setExcluded] = useState<SyndicateKind[]>(["event"]);
  const included = useMemo(
    () => syndicates.filter((syndicate) => !excluded.includes(syndicate.kind)),
    [syndicates, excluded],
  );
  const selection = useSyndicateOffers(included);

  const matches = (syndicate: SyndicateOverview) =>
    search.trim() === ""
      ? null
      : selection.offers.filter(
          (offer) =>
            offer.tag === syndicate.tag &&
            matchesText(offer.search_text, search),
        ).length;
  const tile = (syndicate: SyndicateOverview) => (
    <SyndicateTile
      key={syndicate.tag}
      syndicate={syndicate}
      matches={matches(syndicate)}
    />
  );

  const factions = included.filter(({ kind }) => kind === "faction");
  const others = included.filter(({ kind }) => kind !== "faction");

  return (
    <div className="flex flex-col gap-4">
      <Section
        title="Syndicates"
        action={<ResetChecklistCountdown window={tab.daily_reset} now={now} />}
        controls={
          <div className="flex flex-wrap items-center gap-3 pt-2">
            {KIND_SWITCHES.map(({ kind, label }) => (
              <SwitchField
                key={kind}
                id={`syndicates-exclude-${kind}`}
                label={label}
                checked={excluded.includes(kind)}
                onChange={(checked) =>
                  setExcluded(toggled(excluded, kind, checked))
                }
              />
            ))}
          </div>
        }
      >
        <div className="flex flex-col gap-3">
          {tab.faction_daily && factions.length > 0 && (
            <SyndicateFactionStanding daily={tab.faction_daily}>
              {factions.map(tile)}
            </SyndicateFactionStanding>
          )}
          <SyndicateTileGrid>{others.map(tile)}</SyndicateTileGrid>
        </div>
      </Section>
      <SyndicateOffers
        syndicates={included}
        selection={selection}
        onSearchChange={setSearch}
      />
    </div>
  );
}
