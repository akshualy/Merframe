import { useCallback, useEffect, useMemo, useState } from "react";
import {
  entriesFor,
  type InventoryTabKey,
  inventoryFlags,
} from "@/lib/inventory-entries";
import {
  descending,
  keeps,
  loadFilters,
  saveFilters,
  sortEntries,
  type TabFilters,
} from "@/lib/inventory-filters";
import type { InventoryTab } from "@/types";

export function useInventoryView(
  tab: InventoryTabKey,
  data: InventoryTab | null,
) {
  const [edited, setEdited] = useState<
    Partial<Record<InventoryTabKey, TabFilters>>
  >({});
  const filters = useMemo(() => edited[tab] ?? loadFilters(tab), [edited, tab]);

  const updateFilters = useCallback(
    (patch: Partial<TabFilters>) =>
      setEdited((current) => ({
        ...current,
        [tab]: { ...(current[tab] ?? loadFilters(tab)), ...patch },
      })),
    [tab],
  );

  useEffect(() => {
    saveFilters(tab, filters);
  }, [tab, filters]);

  const entries = useMemo(() => entriesFor(tab, data), [tab, data]);
  const flags = useMemo(() => inventoryFlags(data), [data]);

  const desc = descending(filters);
  const sorted = useMemo(
    () => sortEntries(entries, filters.ordering, desc, tab),
    [entries, filters.ordering, desc, tab],
  );
  const visible = useMemo(
    () => sorted.filter((entry) => keeps(entry, filters, flags)),
    [sorted, filters, flags],
  );

  return { entries, flags, filters, updateFilters, visible };
}
