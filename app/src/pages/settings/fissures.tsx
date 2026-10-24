import { Plus, Trash2 } from "lucide-react";
import { OptionSelect } from "@/components/option-select";
import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { type FilterOption, plainOptions } from "@/lib/filters";
import { occurrenceKeys } from "@/lib/keys";
import { RELIC_TIERS } from "@/lib/relics";
import type { FissureFilter, FissureSubtype, Settings } from "@/types";
import { type PatchAlerts, SwitchRow } from "./row";

const ANY = "all";

const TIERS = plainOptions(RELIC_TIERS);

const MISSION_TYPES: FilterOption[] = [
  { value: "MT_ASSASSINATION", label: "Assassination" },
  { value: "MT_CAPTURE", label: "Capture" },
  { value: "MT_DEFENSE", label: "Defense" },
  { value: "MT_ARTIFACT", label: "Disruption" },
  { value: "MT_EXCAVATE", label: "Excavation" },
  { value: "MT_EXTERMINATION", label: "Extermination" },
  { value: "MT_HIVE", label: "Hive" },
  { value: "MT_RETRIEVAL", label: "Hijack" },
  { value: "MT_TERRITORY", label: "Interception" },
  { value: "MT_MOBILE_DEFENSE", label: "Mobile Defense" },
  { value: "MT_RESCUE", label: "Rescue" },
  { value: "MT_SABOTAGE", label: "Sabotage" },
  { value: "MT_INTEL", label: "Spy" },
  { value: "MT_SURVIVAL", label: "Survival" },
  { value: "MT_ALCHEMY", label: "Alchemy" },
  { value: "MT_ASCENSION", label: "Ascension" },
  { value: "MT_VOID_CASCADE", label: "Void Cascade" },
  { value: "MT_CORRUPTION", label: "Void Flood" },
  { value: "Skirmish", label: "Skirmish" },
  { value: "Volatile", label: "Volatile" },
  { value: "Orphix", label: "Orphix" },
];

const LOCATIONS: FilterOption[] = [
  ...plainOptions([
    "Mercury",
    "Venus",
    "Earth",
    "Lua",
    "Mars",
    "Phobos",
    "Deimos",
    "Ceres",
    "Jupiter",
    "Europa",
    "Saturn",
    "Uranus",
    "Neptune",
    "Pluto",
    "Eris",
    "Sedna",
    "Void",
    "Kuva Fortress",
    "Zariman",
  ]),
  { value: "Veil", label: "Veil Proxima" },
];

const SUBTYPES: FilterOption<FissureSubtype>[] = [
  { value: "normalOrSteelPath", label: "Normal + Steel Path" },
  { value: "normal", label: "Normal" },
  { value: "steelPath", label: "Steel Path" },
  { value: "voidStorm", label: "Void Storm" },
];

function ColumnSelect<T extends string>({
  label,
  value,
  anyLabel,
  options,
  onChange,
}: {
  label: string;
  value: T | typeof ANY;
  anyLabel: string;
  options: readonly FilterOption<T>[];
  onChange: (value: T | typeof ANY) => void;
}) {
  return (
    <div className="flex flex-col gap-1">
      <Label className="text-xs">{label}</Label>
      <OptionSelect<T | typeof ANY>
        label={label}
        value={value}
        options={[{ value: ANY, label: anyLabel }, ...options]}
        onChange={onChange}
        className="w-full"
      />
    </div>
  );
}

export function FissureAlerts({
  draft,
  patchAlerts,
}: {
  draft: Settings;
  patchAlerts: PatchAlerts;
}) {
  const filters = draft.alerts.fissure_filters;
  const filterKeys = occurrenceKeys(
    filters.map((filter) => JSON.stringify(filter)),
  );
  const patchFilter = (index: number, next: Partial<FissureFilter>) =>
    patchAlerts({
      fissure_filters: filters.map((row, position) =>
        position === index ? { ...row, ...next } : row,
      ),
    });

  return (
    <Section
      title="Fissure Alerts"
      description="A fissure or Void Storm notifies when it matches any one of these rows."
    >
      <div className="flex flex-col gap-4">
        <SwitchRow
          checked={draft.alerts.fissure_notifications_enabled}
          onChange={(checked) =>
            patchAlerts({ fissure_notifications_enabled: checked })
          }
        >
          Notify about relic fissures and Void Storms
        </SwitchRow>

        {filters.map((row, index) => (
          <div
            key={filterKeys[index]}
            className="grid items-end gap-3 @sm:grid-cols-[1fr_1fr_1fr_1fr_auto]"
          >
            <ColumnSelect
              label="Tier"
              value={row.tier}
              anyLabel="Any tier"
              options={TIERS}
              onChange={(tier) => patchFilter(index, { tier })}
            />
            <ColumnSelect
              label="Mission"
              value={row.mission}
              anyLabel="Any mission"
              options={MISSION_TYPES}
              onChange={(mission) => patchFilter(index, { mission })}
            />
            <ColumnSelect
              label="Location"
              value={row.location}
              anyLabel="Any planet"
              options={LOCATIONS}
              onChange={(location) => patchFilter(index, { location })}
            />
            <ColumnSelect
              label="Subtype"
              value={row.subtype}
              anyLabel="Any subtype"
              options={SUBTYPES}
              onChange={(subtype) => patchFilter(index, { subtype })}
            />
            <Button
              variant="outline"
              size="icon"
              aria-label="Remove row"
              onClick={() =>
                patchAlerts({
                  fissure_filters: filters.filter(
                    (_, position) => position !== index,
                  ),
                })
              }
            >
              <Trash2 className="size-4" />
            </Button>
          </div>
        ))}

        <Button
          variant="outline"
          size="sm"
          className="self-start"
          disabled={!draft.alerts.fissure_notifications_enabled}
          onClick={() =>
            patchAlerts({
              fissure_filters: [
                ...filters,
                {
                  tier: ANY,
                  mission: ANY,
                  location: ANY,
                  subtype: ANY,
                },
              ],
            })
          }
        >
          <Plus className="size-4" />
          Add Row
        </Button>
      </div>
    </Section>
  );
}
