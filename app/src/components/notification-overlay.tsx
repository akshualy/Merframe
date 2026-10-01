import { type ReactNode, useEffect, useState } from "react";
import Merframe from "@/components/icons/merframe";
import { cn } from "@/lib/utils";
import type { NotificationTrigger } from "@/types";

const HOLD_MS = 5900;

export function NotificationFade({ children }: { children: ReactNode }) {
  const [leaving, setLeaving] = useState(false);

  useEffect(() => {
    const timer = window.setTimeout(() => setLeaving(true), HOLD_MS);
    return () => window.clearTimeout(timer);
  }, []);

  return (
    <div
      className={cn(
        "duration-500 fill-mode-both",
        leaving ? "animate-out fade-out" : "animate-in fade-in",
      )}
    >
      {children}
    </div>
  );
}

export function NotificationOverlay({
  trigger,
}: {
  trigger: NotificationTrigger;
}) {
  return (
    <div className="flex items-center gap-3 px-2 py-1">
      <Merframe className="text-primary size-10 shrink-0" />
      <div className="flex min-w-0 flex-col gap-0.5">
        <p className="truncate text-sm font-semibold">{trigger.title}</p>
        <p className="text-muted-foreground line-clamp-2 text-sm">
          {trigger.body}
        </p>
      </div>
    </div>
  );
}
