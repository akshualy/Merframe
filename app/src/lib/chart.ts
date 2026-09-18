export const WIDTH = 360;
export const HEIGHT = 152;
export const PADDING = { top: 12, right: 10, bottom: 22, left: 46 };
export const INNER_WIDTH = WIDTH - PADDING.left - PADDING.right;
export const INNER_HEIGHT = HEIGHT - PADDING.top - PADDING.bottom;

export const TICK_FONT = 10;
export const TICK_BASELINE = HEIGHT - 6;

const GLYPH_WIDTH = 0.6;
const TICK_CLEARANCE = 10;

export const COMPACT = new Intl.NumberFormat("en-US", {
  notation: "compact",
  maximumFractionDigits: 1,
});

export function tickWidth(text: string): number {
  return text.length * TICK_FONT * GLYPH_WIDTH;
}

export function labelledTicks<T>(
  entries: T[],
  label: (entry: T) => string,
  pitch: number,
  maximum: number,
): { entry: T; index: number }[] {
  const last = entries.length - 1;
  if (last < 0) {
    return [];
  }
  const widest = Math.max(...entries.map((entry) => tickWidth(label(entry))));
  const stride = Math.max(
    1,
    Math.ceil(entries.length / maximum),
    Math.ceil((widest + TICK_CLEARANCE) / pitch),
  );
  return entries
    .map((entry, index) => ({ entry, index }))
    .filter(
      (tick) =>
        tick.index === last ||
        (tick.index % stride === 0 && last - tick.index >= stride),
    );
}

export function tickX(center: number, text: string): number {
  const half = tickWidth(text) / 2;
  return Math.min(Math.max(center, half + 2), WIDTH - half - 2);
}
