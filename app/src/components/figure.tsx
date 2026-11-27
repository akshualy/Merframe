import type { ReactNode } from "react";
import { Hint } from "@/components/ui/hint";
import { cn } from "@/lib/utils";

export function Figures({ children }: { children: ReactNode }) {
  return (
    <div className="flex flex-wrap items-end gap-x-6 gap-y-2">{children}</div>
  );
}

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
      <span className={cn("text-base font-semibold", className)}>{value}</span>
    </span>
  );
}
