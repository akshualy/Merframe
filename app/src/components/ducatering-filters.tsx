import { Field } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { SwitchField } from "@/components/ui/switch";
import type { DucateringHidden, Settings } from "@/types";

const HIDDEN: { value: DucateringHidden; label: string }[] = [
  { value: "listed", label: "Hide items you are selling" },
  { value: "complete_sets", label: "Hide parts of complete sets" },
];

function PlatInput({
  id,
  value,
  disabled,
  onChange,
}: {
  id: string;
  value: number;
  disabled: boolean;
  onChange: (value: number) => void;
}) {
  return (
    <>
      <Input
        id={id}
        type="number"
        min={0}
        disabled={disabled}
        value={value}
        onChange={(event) => onChange(Math.max(0, Number(event.target.value)))}
        className="h-8 w-20 text-right tabular-nums"
      />
      <span className="text-sm">Platinum</span>
    </>
  );
}

export function DucateringFilters({
  settings,
  disabled,
  update,
}: {
  settings: Settings;
  disabled: boolean;
  update: (next: Partial<Settings>) => void;
}) {
  const hidesCompleteSets =
    settings.ducatering_hidden.includes("complete_sets");
  const setHidden = (kind: DucateringHidden, hidden: boolean) =>
    update({
      ducatering_hidden: hidden
        ? [...settings.ducatering_hidden, kind]
        : settings.ducatering_hidden.filter((entry) => entry !== kind),
    });
  return (
    <div className="mt-2 flex flex-wrap items-center gap-x-6 gap-y-2">
      <SwitchField
        id="ducatering-most-ducats"
        label="Most Ducats first"
        checked={settings.ducatering_most_ducats_first}
        disabled={disabled}
        onChange={(checked) =>
          update({
            ducatering_most_ducats_first: checked,
          })
        }
      />
      <Field id="ducatering-max-plat" label="Hide above" inline>
        <PlatInput
          id="ducatering-max-plat"
          value={settings.ducatering_max_plat}
          disabled={disabled}
          onChange={(value) => update({ ducatering_max_plat: value })}
        />
      </Field>
      {HIDDEN.map(({ value, label }) => (
        <SwitchField
          key={value}
          id={`ducatering-${value}`}
          label={label}
          checked={settings.ducatering_hidden.includes(value)}
          disabled={disabled}
          onChange={(checked) => setHidden(value, checked)}
        />
      ))}
      <Field id="ducatering-set-plat" label="Hide sets above" inline>
        <PlatInput
          id="ducatering-set-plat"
          value={settings.ducatering_hidden_set_plat}
          disabled={disabled || !hidesCompleteSets}
          onChange={(value) => update({ ducatering_hidden_set_plat: value })}
        />
      </Field>
    </div>
  );
}
