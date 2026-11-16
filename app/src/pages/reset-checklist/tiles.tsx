import type { ReactNode } from "react";
import { CardGrid, Surface } from "@/components/page";
import { cn } from "@/lib/utils";

export function ResetChecklistTileGrid({ children }: { children: ReactNode }) {
  return (
    <CardGrid className="@3xl:grid-cols-3 @5xl:grid-cols-4 @7xl:grid-cols-5">
      {children}
    </CardGrid>
  );
}

export function ResetChecklistTile({
  className,
  children,
}: {
  className?: string;
  children: ReactNode;
}) {
  return (
    <Surface className={cn("gap-1.5 p-3 text-sm", className)}>
      {children}
    </Surface>
  );
}
