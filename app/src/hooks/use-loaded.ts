import { useEffect, useMemo, useRef, useState } from "react";
import { useListen } from "@/hooks/use-listen";
import type { EventName } from "@/lib/bridge";

export function useLoaded<K, T>(
  key: K | null,
  load: (key: K) => Promise<T>,
  failed: (cause: unknown) => T | null,
  reloadOn: readonly EventName[] = [],
): T | null {
  const [value, setValue] = useState<T | null>(null);
  const [changes, setChanges] = useState(0);
  const latest = useRef({ load, failed });
  latest.current = { load, failed };

  useListen(reloadOn, () => setChanges((count) => count + 1));

  const request = useMemo(
    () => (key === null ? null : { key, changes }),
    [key, changes],
  );

  useEffect(() => {
    if (request === null) {
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
    run(request.key);

    return () => {
      cancelled = true;
    };
  }, [request]);

  return value;
}
