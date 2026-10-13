export type YesNo = "yes" | "no";

export function failsYesNo(mode: YesNo, flag: boolean | null): boolean {
  return (mode === "yes") !== flag;
}

export function passesYesNo<K extends string>(
  filters: Partial<Record<K, YesNo>>,
  has: (key: K) => boolean | null,
): boolean {
  for (const [key, mode] of Object.entries(filters) as [K, YesNo][]) {
    if (failsYesNo(mode, has(key))) {
      return false;
    }
  }

  return true;
}

export interface FilterOption {
  value: string;
  label: string;
}

export function yesNoOptions(yes: string, no: string): FilterOption[] {
  return [
    { value: "yes", label: yes },
    { value: "no", label: no },
  ];
}
