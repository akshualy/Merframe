import { FilterX } from "lucide-react";
import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export function FilterGrid({
  activeFilters = 0,
  onClear,
  className,
  children,
}: {
  activeFilters?: number;
  onClear?: () => void;
  className?: string;
  children: ReactNode;
}) {
  return (
    <div
      className={cn(
        "grid grid-cols-2 gap-x-4 gap-y-3 @lg:grid-cols-3 @5xl:grid-cols-4 @7xl:grid-cols-5",
        className,
      )}
    >
      {children}
      {onClear && activeFilters > 0 && (
        <div className="flex items-end">
          <ClearFilters
            count={activeFilters}
            onClear={onClear}
            className="w-full"
          />
        </div>
      )}
    </div>
  );
}

export function ClearFilters({
  count,
  onClear,
  className,
}: {
  count: number;
  onClear: () => void;
  className?: string;
}) {
  return (
    <Button variant="outline" size="sm" className={className} onClick={onClear}>
      <FilterX className="size-4" />
      Clear {count}
    </Button>
  );
}

export function FilterField({
  label,
  note,
  children,
}: {
  label: string;
  note?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="flex min-w-0 flex-col gap-1">
      <span className="text-muted-foreground flex items-center gap-1 text-xs">
        <span className="truncate">{label}</span>
        {note}
      </span>
      {children}
    </div>
  );
}
