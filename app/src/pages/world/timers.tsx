import { cycleIcon, GameIcon } from "@/components/game-icon";
import { Surface } from "@/components/page";
import { Hint } from "@/components/ui/hint";
import { Progress } from "@/components/ui/progress";
import { secondsUntil } from "@/lib/format";
import { capitalize, elapsedShare, timeLeft } from "@/lib/world";
import type { Timer } from "@/types";

export function TimerChip({ timer, now }: { timer: Timer; now: number }) {
  const icon = cycleIcon(timer.state);
  const remaining = secondsUntil(timer.ends, now);
  return (
    <Surface className="px-4 py-3">
      <div className="flex items-center gap-3">
        <span className="bg-secondary/50 flex size-11 shrink-0 items-center justify-center rounded-full">
          {icon && <GameIcon name={icon} size={28} alt={timer.state} />}
        </span>
        <div className="flex min-w-0 flex-1 flex-col leading-tight">
          <Hint as="span">{timer.name}</Hint>
          <span className="text-primary truncate text-lg font-bold">
            {capitalize(timer.state)}
          </span>
          <Hint as="span">then {timer.next_state}</Hint>
        </div>
        <span className="text-accent font-mono text-base font-semibold tabular-nums">
          {timeLeft(remaining)}
        </span>
      </div>
      <Progress value={elapsedShare(timer.starts, timer.ends, now)} />
    </Surface>
  );
}
