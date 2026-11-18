import type { ReactNode } from "react";
import { Hint } from "@/components/ui/hint";
import { cn } from "@/lib/utils";

export function Figure({
  label,
  value,
  title,
  className,
}: {
  label: string;
  value: ReactNode;
  title?: string;
  className?: string;
}) {
  return (
    <span className="flex flex-col" title={title}>
      <Hint as="span">{label}</Hint>
      <span className={cn("font-mono text-base font-semibold", className)}>
        {value}
      </span>
    </span>
  );
}
