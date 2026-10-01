import { useMemo, useState } from "react";
import { usePreferencesStore } from "@/stores/preferences-store";

export function usePaged<T>(items: T[], resetKey: string) {
  const pageSize = usePreferencesStore((state) => state.pageSize);
  const [position, setPosition] = useState({ page: 0, resetKey });
  if (position.resetKey !== resetKey) {
    setPosition({ page: 0, resetKey });
  }

  const pageCount = Math.max(1, Math.ceil(items.length / pageSize));
  const page =
    position.resetKey === resetKey ? Math.min(position.page, pageCount - 1) : 0;
  const pageItems = useMemo(
    () => items.slice(page * pageSize, (page + 1) * pageSize),
    [items, page, pageSize],
  );

  return {
    pageItems,
    pagination: {
      page,
      pageCount,
      total: items.length,
      onPageChange: (next: number) => setPosition({ page: next, resetKey }),
    },
  };
}
