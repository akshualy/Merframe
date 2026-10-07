import type { ReactNode } from "react";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import type { DucateringHidden, Settings } from "@/types";

const HIDDEN: { value: DucateringHidden; label: string }[] = [
  { value: "listed", label: "Hide items listed on the market" },
  { value: "complete_sets", label: "Hide parts of complete sets" },
];

function Field({
  id,
  label,
  children,
}: {
  id: string;
  label: string;
  children: ReactNode;
}) {
  return (
    <div className="flex items-center gap-2">
      <Label htmlFor={id} className="text-sm">
        {label}
      </Label>
      {children}
    </div>
  );
}

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
      <Field id="ducatering-most-ducats" label="Most Ducats first">
        <Switch
          id="ducatering-most-ducats"
          checked={settings.ducatering_most_ducats_first}
          disabled={disabled}
          onCheckedChange={(checked) =>
            update({ ducatering_most_ducats_first: checked })
          }
        />
      </Field>
      <Field id="ducatering-max-plat" label="Hide above">
        <PlatInput
          id="ducatering-max-plat"
          value={settings.ducatering_max_plat}
          disabled={disabled}
          onChange={(value) => update({ ducatering_max_plat: value })}
        />
      </Field>
      {HIDDEN.map(({ value, label }) => (
        <Field key={value} id={`ducatering-${value}`} label={label}>
          <Switch
            id={`ducatering-${value}`}
            checked={settings.ducatering_hidden.includes(value)}
            disabled={disabled}
            onCheckedChange={(checked) => setHidden(value, checked)}
          />
        </Field>
      ))}
      {hidesCompleteSets && (
        <Field id="ducatering-set-plat-enabled" label="Hide sets above">
          <Switch
            id="ducatering-set-plat-enabled"
            checked={settings.ducatering_hidden_set_plat !== null}
            disabled={disabled}
            onCheckedChange={(checked) =>
              update({ ducatering_hidden_set_plat: checked ? 0 : null })
            }
          />
          <PlatInput
            id="ducatering-set-plat"
            value={settings.ducatering_hidden_set_plat ?? 0}
            disabled={disabled || settings.ducatering_hidden_set_plat === null}
            onChange={(value) => update({ ducatering_hidden_set_plat: value })}
          />
        </Field>
      )}
    </div>
  );
}
