import { Section } from "@/components/page";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { CyclePhase, Settings } from "@/types";
import { CheckboxRow, type PatchAlerts } from "./row";

const TIMERS: {
  world: string;
  phases: { value: CyclePhase; label: string }[];
}[] = [
  {
    world: "Earth",
    phases: [
      { value: "earth_day", label: "Day" },
      { value: "earth_night", label: "Night" },
    ],
  },
  {
    world: "Cetus",
    phases: [
      { value: "cetus_day", label: "Day" },
      { value: "cetus_night", label: "Night" },
    ],
  },
  {
    world: "Orb Vallis",
    phases: [
      { value: "vallis_warm", label: "Warm" },
      { value: "vallis_cold", label: "Cold" },
    ],
  },
  {
    world: "Cambion Drift",
    phases: [
      { value: "cambion_fass", label: "Fass" },
      { value: "cambion_vome", label: "Vome" },
    ],
  },
  {
    world: "Duviri",
    phases: [
      { value: "duviri_sorrow", label: "Sorrow" },
      { value: "duviri_fear", label: "Fear" },
      { value: "duviri_joy", label: "Joy" },
      { value: "duviri_anger", label: "Anger" },
      { value: "duviri_envy", label: "Envy" },
    ],
  },
  {
    world: "Zariman",
    phases: [
      { value: "zariman_corpus", label: "Corpus" },
      { value: "zariman_grineer", label: "Grineer" },
    ],
  },
];

export function CycleTimers({
  draft,
  patchAlerts,
}: {
  draft: Settings;
  patchAlerts: PatchAlerts;
}) {
  return (
    <Section
      title="Cycle Timers"
      description="Notifies once per cycle, before the phase you pick begins."
    >
      <div className="flex flex-col gap-4">
        {TIMERS.map(({ world, phases }) => (
          <div key={world} className="flex flex-col gap-2">
            <span className="text-sm font-semibold">{world}</span>
            <div className="flex flex-wrap gap-x-6 gap-y-2">
              {phases.map(({ value, label }) => (
                <CheckboxRow
                  key={value}
                  checked={draft.alerts.timers.includes(value)}
                  onChange={(checked) =>
                    patchAlerts({
                      timers: checked
                        ? [...draft.alerts.timers, value]
                        : draft.alerts.timers.filter(
                            (phase) => phase !== value,
                          ),
                    })
                  }
                >
                  {label}
                </CheckboxRow>
              ))}
            </div>
          </div>
        ))}
        <div className="flex max-w-64 flex-col gap-2">
          <Label htmlFor="lead">Notify this many minutes ahead</Label>
          <Input
            id="lead"
            type="number"
            min={1}
            max={60}
            value={Math.round(draft.alerts.timer_lead_secs / 60)}
            onChange={(e) =>
              patchAlerts({
                timer_lead_secs: Math.max(1, Number(e.target.value) || 1) * 60,
              })
            }
          />
        </div>
      </div>
    </Section>
  );
}
