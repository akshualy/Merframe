import { Star } from "lucide-react";
import { useCallback, useState } from "react";
import { MarkToggle } from "@/components/mark-toggle";
import { api, reportError } from "@/lib/bridge";

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
    <MarkToggle
      icon={Star}
      pressed={favourite}
      label={favourite ? "Unfavourite" : "Favourite"}
      disabled={busy}
      size={size}
      className={className}
      onClick={handleToggle}
    />
  );
}
