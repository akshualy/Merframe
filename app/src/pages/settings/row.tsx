import type { ReactNode } from "react";
import { Checkbox } from "@/components/ui/checkbox";
import { Hint } from "@/components/ui/hint";
import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";
import type { Settings } from "@/types";

export type Patch = (next: Partial<Settings>) => void;
export type PatchAlerts = (next: Partial<Settings["alerts"]>) => void;

type ToggleProps = {
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
  hint?: ReactNode;
  children: ReactNode;
};

function ToggleRow({
  control,
  hint,
  children,
}: {
  control: ReactNode;
  hint?: ReactNode;
  children: ReactNode;
}) {
  return (
    <label className="flex cursor-pointer items-start gap-3 text-sm">
      {control}
      <span className="flex flex-col gap-1">
        <span>{children}</span>
        {hint && <Hint>{hint}</Hint>}
      </span>
    </label>
  );
}

export function CheckboxRow({
  checked,
  disabled,
  onChange,
  hint,
  children,
}: ToggleProps) {
  return (
    <ToggleRow
      hint={hint}
      control={
        <Checkbox
          className="mt-0.5"
          checked={checked}
          disabled={disabled}
          onCheckedChange={(next) => onChange(next === true)}
        />
      }
    >
      {children}
    </ToggleRow>
  );
}

export function SwitchRow({
  checked,
  disabled,
  onChange,
  hint,
  children,
}: ToggleProps) {
  return (
    <ToggleRow
      hint={hint}
      control={
        <Switch
          checked={checked}
          disabled={disabled}
          onCheckedChange={onChange}
        />
      }
    >
      {children}
    </ToggleRow>
  );
}

export function SliderRow({
  className,
  id,
  label,
  value,
  unit,
  min,
  max,
  step,
  disabled,
  onChange,
  hint,
}: {
  className?: string;
  id: string;
  label: string;
  value: number;
  unit: "min" | "%";
  min: number;
  max: number;
  step: number;
  disabled?: boolean;
  onChange: (value: number) => void;
  hint?: string;
}) {
  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <div className="flex items-baseline justify-between gap-3">
        <Label htmlFor={id}>{label}</Label>
        <span className="text-muted-foreground text-sm">
          {unit === "%" ? `${value}%` : `${value} ${unit}`}
        </span>
      </div>
      <Slider
        id={id}
        min={min}
        max={max}
        step={step}
        value={[value]}
        disabled={disabled}
        onValueChange={([next = value]) => onChange(next)}
      />
      {hint && <Hint>{hint}</Hint>}
    </div>
  );
}
