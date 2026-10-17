import { useCallback, useState } from "react";

export function useFilters<T extends { [K in keyof T]: string | null }>(
  initial: T,
) {
  const [cleared] = useState(initial);
  const [values, setValues] = useState(initial);

  const set = useCallback(
    <K extends keyof T>(key: K, value: T[K]) =>
      setValues((current) => ({ ...current, [key]: value })),
    [],
  );
  const clear = useCallback(() => setValues(cleared), [cleared]);

  const active = Object.values(values).filter((value) => value !== null).length;
  return { values, set, active, clear };
}
