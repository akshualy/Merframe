import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

export function Meta({ className, ...props }: ComponentProps<"span">) {
  return (
    <span className={cn("flex items-center gap-1", className)} {...props} />
  );
}
