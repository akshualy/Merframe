import { Plus, Trash2 } from "lucide-react";
import { EmptyNote, Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { occurrenceKeys } from "@/lib/keys";
import { RELIC_TIERS } from "@/lib/relics";
import type { FissureFilter, Settings, SteelPathFilter } from "@/types";
import { CheckboxRow, type PatchAlerts } from "./row";

const ANY = "all";

const MISSION_TYPES: { value: string; label: string }[] = [
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
];

const PLANETS = [
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
];

const STEEL_PATH: { value: SteelPathFilter; label: string }[] = [
  { value: "all", label: "Either" },
  { value: "steelPath", label: "Steel Path only" },
  { value: "normal", label: "Normal only" },
];

function ColumnSelect({
  value,
  onChange,
  anyLabel,
  options,
}: {
  value: string;
  onChange: (value: string) => void;
  anyLabel: string;
  options: { value: string; label: string }[];
}) {
  return (
    <Select value={value} onValueChange={onChange}>
      <SelectTrigger className="w-full">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value={ANY}>{anyLabel}</SelectItem>
        {options.map(({ value, label }) => (
          <SelectItem key={value} value={value}>
            {label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
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
      description="A fissure notifies when it matches any one of these rows."
      action={
        <Button
          variant="outline"
          size="sm"
          onClick={() =>
            patchAlerts({
              fissure_filters: [
                ...filters,
                { tier: ANY, mission: ANY, location: ANY, steel_path: "all" },
              ],
            })
          }
        >
          <Plus className="size-4" />
          Add Row
        </Button>
      }
    >
      <div className="flex flex-col gap-4">
        <CheckboxRow
          checked={draft.alerts.fissure_notifications_enabled}
          onChange={(checked) =>
            patchAlerts({ fissure_notifications_enabled: checked })
          }
        >
          Notify about relic fissures
        </CheckboxRow>

        {filters.length === 0 && (
          <EmptyNote>
            No rows yet. A column left on &quot;Any&quot; accepts everything.
          </EmptyNote>
        )}

        {filters.map((row, index) => (
          <div
            key={filterKeys[index]}
            className="grid items-end gap-3 sm:grid-cols-[1fr_1fr_1fr_1fr_auto]"
          >
            <div className="flex flex-col gap-1">
              <Label className="text-xs">Tier</Label>
              <ColumnSelect
                value={row.tier}
                anyLabel="Any tier"
                options={RELIC_TIERS.map((tier) => ({
                  value: tier,
                  label: tier,
                }))}
                onChange={(tier) => patchFilter(index, { tier })}
              />
            </div>
            <div className="flex flex-col gap-1">
              <Label className="text-xs">Mission</Label>
              <ColumnSelect
                value={row.mission}
                anyLabel="Any mission"
                options={MISSION_TYPES}
                onChange={(mission) => patchFilter(index, { mission })}
              />
            </div>
            <div className="flex flex-col gap-1">
              <Label className="text-xs">Location</Label>
              <ColumnSelect
                value={row.location}
                anyLabel="Any planet"
                options={PLANETS.map((planet) => ({
                  value: planet,
                  label: planet,
                }))}
                onChange={(location) => patchFilter(index, { location })}
              />
            </div>
            <div className="flex flex-col gap-1">
              <Label className="text-xs">Steel Path</Label>
              <Select
                value={row.steel_path}
                onValueChange={(value) =>
                  patchFilter(index, {
                    steel_path: value as SteelPathFilter,
                  })
                }
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {STEEL_PATH.map(({ value, label }) => (
                    <SelectItem key={value} value={value}>
                      {label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
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
      </div>
    </Section>
  );
}
