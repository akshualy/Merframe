import { useNavigate } from "react-router";
import { GameIcon } from "@/components/game-icon";
import { Surface } from "@/components/page";
import { Hint } from "@/components/ui/hint";
import { useAsyncData } from "@/hooks/use-async-data";
import {
  ARBITRATION_RELOAD_EVENTS,
  currentSlot,
  liveSlots,
  nextSlot,
  slotMission,
  slotPlace,
} from "@/lib/arbitrations";
import { api } from "@/lib/bridge";
import { secondsUntil } from "@/lib/format";
import { cn } from "@/lib/utils";
import { endingSoon, timeLeft } from "@/lib/world";
import { TierBadge } from "@/pages/arbitrations/schedule";

export function WorldArbitrationCard({ now }: { now: number }) {
  const { data } = useAsyncData(
    "arbitrations",
    api.arbitrations,
    ARBITRATION_RELOAD_EVENTS,
  );
  const navigate = useNavigate();
  const slots = liveSlots(data?.schedule ?? null, now);
  const current = currentSlot(slots, now);
  const next = nextSlot(slots, now);
  const remaining = current ? secondsUntil(current.ends, now) : 0;
  return (
    <Surface
      className="cursor-pointer px-4 py-3 @sm:col-span-full"
      role="link"
      tabIndex={0}
      title="Open the Arbitration schedule"
      onClick={() => navigate("/arbitrations")}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          navigate("/arbitrations");
        }
      }}
    >
      <div className="flex items-center gap-3">
        <span className="bg-secondary/50 flex size-11 shrink-0 items-center justify-center rounded-full">
          <GameIcon name="arbitration" size={28} alt="" />
        </span>
        <div className="flex min-w-0 flex-1 flex-col leading-tight">
          <Hint as="span">Arbitration</Hint>
          {current ? (
            <>
              <span className="text-primary truncate text-lg font-bold">
                {slotMission(current)}
              </span>
              <Hint as="span" className="truncate">
                {slotPlace(current)}
              </Hint>
            </>
          ) : (
            <span className="truncate text-sm">
              {data?.status === "unavailable"
                ? "Forecast unavailable"
                : "Loading the forecast"}
            </span>
          )}
        </div>
        {current && (
          <>
            <TierBadge tier={current.tier} />
            <span
              className={cn(
                "font-mono text-base font-semibold",
                endingSoon(remaining) ? "text-warning" : "text-accent",
              )}
            >
              {timeLeft(remaining)}
            </span>
          </>
        )}
      </div>
      {next && (
        <Hint as="span" className="truncate">
          Next: {slotMission(next)}, {slotPlace(next)}
        </Hint>
      )}
    </Surface>
  );
}
