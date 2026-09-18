export function occurrenceKeys(names: string[]): string[] {
  const seen = new Map<string, number>();
  return names.map((name) => {
    const occurrence = seen.get(name) ?? 0;
    seen.set(name, occurrence + 1);
    return `${name}#${occurrence}`;
  });
}
