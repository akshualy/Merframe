import { FilterField } from "@/components/filter-grid";
import { OptionSelect } from "@/components/option-select";
import type { FilterOption } from "@/lib/filters";
import { cn } from "@/lib/utils";

export function FilterSingleSelect<T extends string>({
  label,
  value,
  options,
  onChange,
  disabled,
}: {
  label: string;
  value: T;
  options: readonly FilterOption<T>[];
  onChange: (value: T) => void;
  disabled?: boolean;
}) {
  return (
    <FilterField label={label}>
      <OptionSelect
        label={label}
        value={value}
        options={options}
        onChange={onChange}
        size="sm"
        dense
        className="w-full"
        disabled={disabled}
      />
    </FilterField>
  );
}

export function FilterSelect<T extends string>({
  label,
  value,
  options,
  onChange,
  anyLabel = "Any",
  disabled,
}: {
  label: string;
  value: T | null;
  options: readonly FilterOption<T>[];
  onChange: (value: T | null) => void;
  anyLabel?: string;
  disabled?: boolean;
}) {
  return (
    <FilterField label={label}>
      <OptionSelect
        label={label}
        value={value}
        options={options}
        onChange={onChange}
        anyLabel={anyLabel}
        size="sm"
        dense
        className={cn("w-full", value === null && "text-muted-foreground")}
        disabled={disabled}
      />
    </FilterField>
  );
}
