import { Search } from "lucide-react";
import type { ComponentProps } from "react";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

export function SearchInput({
  className,
  ...props
}: ComponentProps<typeof Input>) {
  return (
    <div className="relative w-64">
      <Search className="text-muted-foreground pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2" />
      <Input {...props} className={cn("pl-9", className)} />
    </div>
  );
}
