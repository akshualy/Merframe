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

export function MinutesSlider({
  className,
  id,
  label,
  value,
  min,
  max,
  step,
  onChange,
  hint,
}: {
  className?: string;
  id: string;
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (minutes: number) => void;
  hint?: string;
}) {
  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <div className="flex items-baseline justify-between gap-3">
        <Label htmlFor={id}>{label}</Label>
        <span className="text-muted-foreground text-sm tabular-nums">
          {value} min
        </span>
      </div>
      <Slider
        id={id}
        min={min}
        max={max}
        step={step}
        value={[value]}
        onValueChange={([minutes = value]) => onChange(minutes)}
      />
      {hint && <Hint>{hint}</Hint>}
    </div>
  );
}
