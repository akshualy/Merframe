import { useEffect, useRef, useState } from "react";

export function useLoaded<K, T>(
  key: K | null,
  load: (key: K) => Promise<T>,
  failed: (cause: unknown) => T | null,
): T | null {
  const [value, setValue] = useState<T | null>(null);
  const latest = useRef({ load, failed });
  latest.current = { load, failed };

  useEffect(() => {
    if (key === null) {
      setValue(null);
      return;
    }
    let cancelled = false;
    async function run(current: K) {
      try {
        const next = await latest.current.load(current);
        if (!cancelled) {
          setValue(next);
        }
      } catch (error) {
        if (!cancelled) {
          setValue(latest.current.failed(error));
        }
      }
    }
    run(key);
    return () => {
      cancelled = true;
    };
  }, [key]);

  return value;
}
