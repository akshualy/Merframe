import { Star } from "lucide-react";
import { useCallback, useState } from "react";
import { Button } from "@/components/ui/button";
import { api, reportError } from "@/lib/bridge";
import { cn } from "@/lib/utils";

export function FavouriteStar({
  uniqueName,
  favourite,
  size = "default",
  className,
}: {
  uniqueName: string;
  favourite: boolean;
  size?: "default" | "small";
  className?: string;
}) {
  const [busy, setBusy] = useState(false);

  const handleToggle = useCallback(
    async (e: React.MouseEvent) => {
      e.stopPropagation();
      setBusy(true);
      try {
        await api.toggleFavourite(uniqueName);
      } catch (error) {
        reportError(error);
      } finally {
        setBusy(false);
      }
    },
    [uniqueName],
  );

  return (
    <Button
      variant="ghost"
      size="icon"
      disabled={busy}
      aria-label={favourite ? "Unfavourite" : "Favourite"}
      aria-pressed={favourite}
      title={favourite ? "Unfavourite" : "Favourite"}
      className={cn(
        "shrink-0",
        size === "small" ? "size-6" : "size-7",
        favourite
          ? "text-accent hover:text-accent"
          : "text-muted-foreground/50 hover:text-accent",
        className,
      )}
      onClick={handleToggle}
    >
      <Star
        className={size === "small" ? "size-3.5" : "size-4"}
        fill={favourite ? "currentColor" : "none"}
      />
    </Button>
  );
}
