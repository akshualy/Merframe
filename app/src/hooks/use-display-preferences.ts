import { useEffect } from "react";
import type { Motion, TextSize } from "@/types";

const REDUCED_MOTION = "(prefers-reduced-motion: reduce)";

export function useDisplayPreferences({
  text_size,
  motion,
}: {
  text_size: TextSize;
  motion: Motion;
}) {
  useEffect(() => {
    document.documentElement.dataset.textSize = text_size;
  }, [text_size]);

  useEffect(() => {
    const root = document.documentElement;
    const query = window.matchMedia(REDUCED_MOTION);
    const apply = () => {
      const reduced =
        motion === "system" ? query.matches : motion === "reduced";
      root.dataset.motion = reduced ? "reduced" : "full";
    };

    apply();
    query.addEventListener("change", apply);
    return () => query.removeEventListener("change", apply);
  }, [motion]);
}
