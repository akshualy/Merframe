import { Field } from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ANY, type FilterOption } from "@/lib/filters";
import { cn } from "@/lib/utils";

type Choice<T extends string> =
  | {
      value: NoInfer<T>;
      onChange: (value: NoInfer<T>) => void;
      anyLabel?: undefined;
    }
  | {
      value: NoInfer<T> | null;
      onChange: (value: NoInfer<T> | null) => void;
      anyLabel: string;
    };

type OptionSelectProps<T extends string> = Choice<T> & {
  id?: string;
  label?: string;
  options: readonly FilterOption<T>[];
  size?: "sm" | "default";
  dense?: boolean;
  className?: string;
  disabled?: boolean;
};

export function OptionSelect<T extends string>(props: OptionSelectProps<T>) {
  const { id, label, options, size, dense, className, disabled } = props;
  const items =
    props.anyLabel === undefined
      ? options
      : [{ value: ANY, label: props.anyLabel }, ...options];

  const choose = (next: string) => {
    if (props.anyLabel === undefined) {
      props.onChange(next as T);
    } else {
      props.onChange(next === ANY ? null : (next as T));
    }
  };

  return (
    <Select
      value={props.value ?? ANY}
      onValueChange={choose}
      disabled={disabled}
    >
      <SelectTrigger
        id={id}
        size={size}
        aria-label={label}
        className={cn(dense && "text-xs", className)}
      >
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {items.map((option) => (
          <SelectItem
            key={option.value}
            value={option.value}
            className={cn(dense && "text-xs")}
          >
            {option.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

export function SelectField<T extends string>({
  id,
  label,
  inline = false,
  ...select
}: OptionSelectProps<T> & { id: string; label: string; inline?: boolean }) {
  return (
    <Field id={id} label={label} inline={inline}>
      <OptionSelect<T> id={id} {...select} />
    </Field>
  );
}
