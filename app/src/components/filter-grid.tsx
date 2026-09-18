import { FilterX } from "lucide-react";
import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";

export function FilterGrid({
  activeFilters = 0,
  onClear,
  children,
}: {
  activeFilters?: number;
  onClear?: () => void;
  children: ReactNode;
}) {
  return (
    <div className="grid grid-cols-2 gap-x-4 gap-y-3 rounded-xl border p-3 md:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5">
      {children}
      {onClear && activeFilters > 0 && (
        <div className="flex items-end">
          <Button
            variant="outline"
            size="sm"
            className="w-full"
            onClick={onClear}
          >
            <FilterX className="size-4" />
            Clear {activeFilters}
          </Button>
        </div>
      )}
    </div>
  );
}

export function FilterField({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <div className="flex min-w-0 flex-col gap-1">
      <span className="text-muted-foreground truncate text-xs">{label}</span>
      {children}
    </div>
  );
}
