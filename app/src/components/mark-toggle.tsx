import type { LucideIcon } from "lucide-react";
import type { MouseEvent } from "react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export function MarkToggle({
  icon: Icon,
  pressed,
  label,
  disabled,
  size = "default",
  className,
  onClick,
}: {
  icon: LucideIcon;
  pressed: boolean;
  label: string;
  disabled?: boolean;
  size?: "default" | "small";
  className?: string;
  onClick: (event: MouseEvent) => void;
}) {
  return (
    <Button
      variant="ghost"
      size="icon"
      disabled={disabled}
      aria-label={label}
      aria-pressed={pressed}
      title={label}
      className={cn(
        "shrink-0 hover:bg-transparent dark:hover:bg-transparent",
        size === "small" ? "size-6" : "size-7",
        pressed
          ? "text-accent hover:text-accent"
          : "text-muted-foreground/50 hover:text-accent",
        className,
      )}
      onClick={onClick}
    >
      <Icon
        className={size === "small" ? "size-3.5" : "size-4"}
        fill={pressed ? "currentColor" : "none"}
      />
    </Button>
  );
}
