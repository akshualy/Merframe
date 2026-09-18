import type { ReactNode } from "react";
import { Checkbox } from "@/components/ui/checkbox";
import { Hint } from "@/components/ui/hint";
import type { Settings } from "@/types";

export type Patch = (next: Partial<Settings>) => void;
export type PatchAlerts = (next: Partial<Settings["alerts"]>) => void;

export function CheckboxRow({
  checked,
  disabled,
  onChange,
  hint,
  children,
}: {
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
  hint?: ReactNode;
  children: ReactNode;
}) {
  return (
    <label className="flex cursor-pointer items-start gap-3 text-sm">
      <Checkbox
        className="mt-0.5"
        checked={checked}
        disabled={disabled}
        onCheckedChange={(next) => onChange(next === true)}
      />
      <span className="flex flex-col gap-1">
        <span>{children}</span>
        {hint && <Hint>{hint}</Hint>}
      </span>
    </label>
  );
}
