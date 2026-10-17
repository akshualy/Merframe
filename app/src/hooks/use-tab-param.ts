import { useCallback } from "react";
import { useSearchParams } from "react-router";

export function useTabParam<T extends string>(
  fallback: T,
): [T, (tab: string) => void] {
  const [params, setParams] = useSearchParams();
  const tab = (params.get("tab") as T | null) ?? fallback;
  const setTab = useCallback(
    (next: string) => setParams({ tab: next }, { replace: true }),
    [setParams],
  );
  return [tab, setTab];
}
