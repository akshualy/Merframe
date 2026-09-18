import type { ComponentProps } from "react";

import { cn } from "@/lib/utils";

export function Hint({
  as: Tag = "p",
  className,
  ...props
}: ComponentProps<"p"> & { as?: "p" | "span" | "dt" }) {
  return (
    <Tag
      className={cn("text-muted-foreground text-xs", className)}
      {...props}
    />
  );
}
