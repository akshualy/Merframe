import { Trash2 } from "lucide-react";
import { useMemo } from "react";
import { ItemImage } from "@/components/item-image";
import { ItemSearch } from "@/components/item-search";
import { EmptyNote, Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import { Label } from "@/components/ui/label";
import { useAsyncData } from "@/hooks/use-async-data";
import { api } from "@/lib/bridge";
import type {
  Settings,
  WorldRewardAlertChoices,
  WorldRewardAlertGroup,
} from "@/types";
import { CheckboxRow, type PatchAlerts, SwitchRow } from "./row";

const GROUPS: { value: WorldRewardAlertGroup; label: string }[] = [
  { value: "catalysts", label: "Orokin Catalysts" },
  { value: "reactors", label: "Orokin Reactors" },
  { value: "exilus_adapters", label: "Exilus Adapters" },
  { value: "unmastered_weapons", label: "Weapons you still need for mastery" },
  {
    value: "unusual_weapons",
    label: "Weapons outside the usual invasion pool",
  },
];

function WeaponParts({
  chosen,
  onChange,
}: {
  chosen: string[];
  onChange: (next: string[]) => void;
}) {
  const { data: parts } = useAsyncData(
    "invasion-weapon-parts",
    api.invasionWeaponParts,
  );
  const byUniqueName = useMemo(
    () => new Map((parts ?? []).map((part) => [part.unique_name, part])),
    [parts],
  );
  const open = (parts ?? []).filter(
    (part) => !chosen.includes(part.unique_name),
  );

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-1">
        <Label>Weapon parts</Label>
        <Hint>Parts of the weapons invasions give.</Hint>
      </div>
      <ItemSearch
        items={open}
        placeholder="Search weapon parts"
        onPick={(part) => onChange([...chosen, part.unique_name])}
      />
      {chosen.length === 0 ? (
        <EmptyNote>No weapon part chosen.</EmptyNote>
      ) : (
        <ul className="flex flex-col gap-2">
          {chosen.map((uniqueName) => {
            const part = byUniqueName.get(uniqueName);
            return (
              <li key={uniqueName} className="flex items-center gap-2">
                <ItemImage
                  imageName={part?.image_name}
                  size={32}
                  alt={part?.name}
                />
                <span className="flex-1 truncate font-semibold">
                  {part?.name}
                </span>
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label="Remove weapon part"
                  title="Remove"
                  onClick={() =>
                    onChange(chosen.filter((kept) => kept !== uniqueName))
                  }
                >
                  <Trash2 className="size-4" />
                </Button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

export function WorldRewardAlertSettings({
  draft,
  patchAlerts,
}: {
  draft: Settings;
  patchAlerts: PatchAlerts;
}) {
  const rewards = draft.alerts.rewards;
  const patchRewards = (next: Partial<WorldRewardAlertChoices>) =>
    patchAlerts({ rewards: { ...rewards, ...next } });

  return (
    <Section
      title="Reward Alerts"
      description="Notifies when an alert, invasion or event has one of these rewards. The World page lists any matching missions."
    >
      <div className="flex flex-col gap-4">
        <SwitchRow
          checked={draft.alerts.reward_notifications_enabled}
          onChange={(checked) =>
            patchAlerts({ reward_notifications_enabled: checked })
          }
        >
          Notify about chosen rewards
        </SwitchRow>
        <div className="flex flex-wrap gap-x-6 gap-y-2">
          {GROUPS.map(({ value, label }) => (
            <CheckboxRow
              key={value}
              checked={rewards.groups.includes(value)}
              onChange={(checked) =>
                patchRewards({
                  groups: checked
                    ? [...rewards.groups, value]
                    : rewards.groups.filter((group) => group !== value),
                })
              }
            >
              {label}
            </CheckboxRow>
          ))}
        </div>
        <WeaponParts
          chosen={rewards.weapon_parts}
          onChange={(weapon_parts) => patchRewards({ weapon_parts })}
        />
      </div>
    </Section>
  );
}
