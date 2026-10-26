import { type LucideIcon, Timer } from "lucide-react";
import type { ReactNode } from "react";
import { GameIcon } from "@/components/game-icon";
import { EmptyNote, FramedList, Surface } from "@/components/page";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import { Progress } from "@/components/ui/progress";
import {
  type ArbitrationDay,
  slotMission,
  slotPlace,
} from "@/lib/arbitrations";
import { secondsUntil, timeOfDay } from "@/lib/format";
import { cn } from "@/lib/utils";
import { elapsedShare, endingSoon, timeLeft } from "@/lib/world";
import type { ArbitrationSlot, ArbitrationTier } from "@/types";

const TIER_VARIANT: Record<ArbitrationTier, "accent" | "secondary" | "muted"> =
  {
    S: "accent",
    A: "accent",
    B: "secondary",
    C: "secondary",
    D: "muted",
    F: "muted",
  };

export function TierBadge({ tier }: { tier: ArbitrationTier }) {
  return (
    <Badge
      variant={TIER_VARIANT[tier]}
      className="w-8 justify-center"
      title={`Community tier ${tier}`}
    >
      {tier}
    </Badge>
  );
}

function HeroHeading({
  icon: Icon,
  label,
  children,
}: {
  icon: LucideIcon | "arbitration";
  label: string;
  children?: ReactNode;
}) {
  return (
    <div className="flex items-center gap-2">
      {Icon === "arbitration" ? (
        <GameIcon name="arbitration" size={16} alt="" />
      ) : (
        <Icon className="text-primary size-4" />
      )}
      <Hint as="span" className="flex-1 font-medium">
        {label}
      </Hint>
      {children}
    </div>
  );
}

function HeroCard({
  icon,
  label,
  slot,
  empty,
  children,
}: {
  icon: LucideIcon | "arbitration";
  label: string;
  slot: ArbitrationSlot | null;
  empty: { title: string; note: string };
  children?: ReactNode;
}) {
  return (
    <Surface className="gap-1 px-4 py-3">
      <HeroHeading icon={icon} label={label}>
        {slot && <TierBadge tier={slot.tier} />}
      </HeroHeading>
      {slot ? (
        <>
          <span className="text-primary truncate text-lg font-bold">
            {slotMission(slot)}
          </span>
          <span className="truncate text-sm">{slotPlace(slot)}</span>
          <Hint as="span">{slot.faction ?? "Unknown faction"}</Hint>
        </>
      ) : (
        <>
          <span className="text-lg font-bold">{empty.title}</span>
          <Hint as="span">{empty.note}</Hint>
        </>
      )}
      {children && (
        <div className="mt-auto flex flex-col gap-1 pt-2">{children}</div>
      )}
    </Surface>
  );
}

function Countdown({ label, seconds }: { label: string; seconds: number }) {
  return (
    <span className="flex flex-col">
      <Hint as="span">{label}</Hint>
      <span
        className={cn(
          "font-mono text-base font-semibold",
          endingSoon(seconds) ? "text-warning" : "text-accent",
        )}
      >
        {timeLeft(seconds)}
      </span>
    </span>
  );
}

export function HeroCards({
  current,
  next,
  now,
}: {
  current: ArbitrationSlot | null;
  next: ArbitrationSlot | null;
  now: number;
}) {
  return (
    <div className="grid gap-3 @lg:grid-cols-2">
      <HeroCard
        icon="arbitration"
        label="Current rotation"
        slot={current}
        empty={{
          title: "No current forecast",
          note: "The schedule refreshes on its own.",
        }}
      >
        {current && (
          <>
            <Countdown
              label="Time left to enter"
              seconds={secondsUntil(current.ends, now)}
            />
            <Progress value={elapsedShare(current.starts, current.ends, now)} />
          </>
        )}
      </HeroCard>
      <HeroCard
        icon={Timer}
        label="Up next"
        slot={next}
        empty={{
          title: "Next hour unknown",
          note: "The forecast has no later rotation.",
        }}
      >
        {next && (
          <span className="flex flex-col">
            <Hint as="span">Starts at</Hint>
            <span className="text-accent font-mono text-base font-semibold">
              {timeOfDay(next.starts)}
            </span>
          </span>
        )}
      </HeroCard>
    </div>
  );
}

function SlotRow({ slot, now }: { slot: ArbitrationSlot; now: number }) {
  const started = new Date(slot.starts).getTime() <= now;
  return (
    <li
      className={cn(
        "flex items-center gap-3 px-3 py-2",
        started && "bg-secondary/40",
      )}
    >
      <span className="flex w-24 shrink-0 flex-col font-mono text-sm">
        <span className="font-semibold">{timeOfDay(slot.starts)}</span>
        {started ? (
          <Hint as="span" className="text-accent">
            Current
          </Hint>
        ) : (
          <Hint as="span">to {timeOfDay(slot.ends)}</Hint>
        )}
      </span>
      <TierBadge tier={slot.tier} />
      <span className="min-w-0 flex-1 truncate text-sm">
        <span className="font-semibold">{slotMission(slot)}</span>
        <span className="text-muted-foreground"> ({slotPlace(slot)})</span>
      </span>
      <Hint as="span" className="hidden w-24 shrink-0 @md:block">
        {slot.faction ?? "Unknown faction"}
      </Hint>
    </li>
  );
}

export function DayList({
  days,
  total,
  now,
  onClear,
}: {
  days: ArbitrationDay[];
  total: number;
  now: number;
  onClear: () => void;
}) {
  if (total === 0) {
    return <EmptyNote>The forecast is not loaded yet.</EmptyNote>;
  }

  if (days.length === 0) {
    return (
      <div className="flex flex-col items-start gap-3">
        <EmptyNote>No rotation matches these choices.</EmptyNote>
        <Button variant="outline" size="sm" onClick={onClear}>
          Clear search and filters
        </Button>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-3">
      {days.map((day) => (
        <FramedList
          key={day.key}
          header={
            <>
              <span className="text-sm font-semibold">{day.label}</span>
              <Hint as="span">{day.date}</Hint>
            </>
          }
        >
          {day.slots.map((slot) => (
            <SlotRow key={slot.id} slot={slot} now={now} />
          ))}
        </FramedList>
      ))}
    </div>
  );
}
