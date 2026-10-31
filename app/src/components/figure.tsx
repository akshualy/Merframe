import { Hint } from "@/components/ui/hint";
import { cn } from "@/lib/utils";

export function Figure({
  label,
  value,
  className,
}: {
  label: string;
  value: string;
  className?: string;
}) {
  return (
    <span className="flex flex-col">
      <Hint as="span">{label}</Hint>
      <span className={cn("font-mono text-base font-semibold", className)}>
        {value}
      </span>
    </span>
  );
}
