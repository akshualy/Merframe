import { FilterField } from "@/components/filter-grid";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { FilterOption } from "@/lib/filters";
import { cn } from "@/lib/utils";

const ANY_VALUE = "any";

export function FilterSelect({
  label,
  value,
  options,
  onChange,
  anyLabel = "Any",
}: {
  label: string;
  value: string | null;
  options: readonly FilterOption[];
  onChange: (value: string | null) => void;
  anyLabel?: string;
}) {
  return (
    <FilterField label={label}>
      <Select
        value={value ?? ANY_VALUE}
        onValueChange={(next) => onChange(next === ANY_VALUE ? null : next)}
      >
        <SelectTrigger
          size="sm"
          aria-label={label}
          className={cn(
            "w-full text-xs",
            value === null && "text-muted-foreground",
          )}
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={ANY_VALUE} className="text-xs">
            {anyLabel}
          </SelectItem>
          {options.map((option) => (
            <SelectItem
              key={option.value}
              value={option.value}
              className="text-xs"
            >
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </FilterField>
  );
}

export function FilterSingleSelect<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: readonly { value: T; label: string }[];
  onChange: (value: T) => void;
}) {
  return (
    <FilterField label={label}>
      <Select value={value} onValueChange={(next) => onChange(next as T)}>
        <SelectTrigger size="sm" aria-label={label} className="w-full text-xs">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {options.map((option) => (
            <SelectItem
              key={option.value}
              value={option.value}
              className="text-xs"
            >
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </FilterField>
  );
}
