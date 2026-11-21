import { useMemo } from "react";
import { useSettings } from "@/stores/app-store";
import type { SaleSafetySettings } from "@/types";

export function useSaleSafetySettings(): SaleSafetySettings {
  const settings = useSettings();
  const keep = settings.sale_safety_keep;
  const favourites = settings.sale_safety_keep_favourites;
  return useMemo(
    () => ({ sale_safety_keep: keep, sale_safety_keep_favourites: favourites }),
    [keep, favourites],
  );
}
