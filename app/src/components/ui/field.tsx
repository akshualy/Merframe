import type { ReactNode } from "react";

import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";

function Field({
  id,
  label,
  inline = false,
  children,
}: {
  id: string;
  label: ReactNode;
  inline?: boolean;
  children: ReactNode;
}) {
  return (
    <div className={inline ? "flex items-center gap-2" : "flex flex-col gap-2"}>
      <Label htmlFor={id} className={cn(inline && "text-sm")}>
        {label}
      </Label>
      {children}
    </div>
  );
}

export { Field };
