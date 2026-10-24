import { Gift } from "lucide-react";
import { ItemImage } from "@/components/item-image";
import { EmptyNote } from "@/components/page";
import { secondsUntil } from "@/lib/format";
import { timeLeft } from "@/lib/world";
import type { WorldRewardAlert } from "@/types";
import { Group, InfoRow, Panel } from "./panels";

function rewardHint(reward: WorldRewardAlert): string {
  const where = [reward.source, reward.mission, reward.location, reward.side]
    .filter((part) => part !== null)
    .join(", ");
  return reward.requirement ? `${where}. ${reward.requirement}` : where;
}

export function WorldRewardAlertsPanel({
  rewards,
  now,
}: {
  rewards: WorldRewardAlert[];
  now: number;
}) {
  return (
    <Panel icon={Gift} title="Rewards on Offer">
      {rewards.length === 0 ? (
        <EmptyNote>
          No alert, invasion or event offers one of your chosen rewards right
          now. Pick them under Settings.
        </EmptyNote>
      ) : (
        <Group>
          {rewards.map((reward) => (
            <div key={reward.key} className="flex items-center gap-3">
              <ItemImage imageName={reward.image_name} size={32} />
              <div className="min-w-0 flex-1">
                <InfoRow
                  label={reward.label}
                  hint={rewardHint(reward)}
                  value={
                    reward.expiry
                      ? timeLeft(secondsUntil(reward.expiry, now))
                      : "Ongoing"
                  }
                />
              </div>
            </div>
          ))}
        </Group>
      )}
    </Panel>
  );
}
