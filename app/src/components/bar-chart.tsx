import {
  ChartHeader,
  ChartTooltip,
  useChartHover,
} from "@/components/chart-hover";
import {
  COMPACT,
  HEIGHT,
  INNER_HEIGHT,
  INNER_WIDTH,
  labelledTicks,
  PADDING,
  TICK_BASELINE,
  TICK_FONT,
  tickX,
  WIDTH,
} from "@/lib/chart";
import { cn } from "@/lib/utils";

const MAX_TICKS = 4;
const GAP = 2;
const MAX_BAR_WIDTH = 24;
const DIM = 0.35;

export interface Bar {
  key: string;
  label: string;
  value: number;
}

export function BarChart({
  label,
  color,
  bars,
  format,
  className,
}: {
  label: string;
  color: string;
  bars: Bar[];
  format: (value: number) => string;
  className?: string;
}) {
  const high = Math.max(1, ...bars.map((bar) => bar.value));
  const slot = bars.length === 0 ? INNER_WIDTH : INNER_WIDTH / bars.length;
  const width = Math.min(MAX_BAR_WIDTH, Math.max(slot * 0.6, slot - GAP));
  const total = bars.reduce((sum, bar) => sum + bar.value, 0);
  const hover = useChartHover(bars.length, (x) => {
    const index = Math.floor((x - PADDING.left) / slot);
    return index < 0 || index >= bars.length ? null : index;
  });
  const hovered = hover.index === null ? null : (bars[hover.index] ?? null);
  const centreOf = (index: number) => PADDING.left + index * slot + slot / 2;

  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <ChartHeader
        color={color}
        label={label}
        note={`peak ${format(high)} a day`}
        value={format(total)}
        caption={
          bars.length === 0
            ? "no days in this window"
            : `over ${format(bars.length)} days`
        }
      />
      <div className="relative">
        <svg
          viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
          className="w-full"
          role="img"
          aria-label={`${label} as one bar per day`}
          {...hover.handlers}
        >
          <title>{label}</title>
          {[0, 0.5, 1].map((fraction) => (
            <line
              key={fraction}
              x1={PADDING.left}
              x2={WIDTH - PADDING.right}
              y1={PADDING.top + INNER_HEIGHT * fraction}
              y2={PADDING.top + INNER_HEIGHT * fraction}
              stroke="currentColor"
              strokeOpacity={0.1}
            />
          ))}
          {[high, 0].map((value) => (
            <text
              key={value}
              x={PADDING.left - 6}
              y={PADDING.top + INNER_HEIGHT - (value / high) * INNER_HEIGHT + 3}
              textAnchor="end"
              fontSize={TICK_FONT}
              fill="currentColor"
              fillOpacity={0.6}
            >
              {COMPACT.format(value)}
            </text>
          ))}
          {bars.map((bar, index) => {
            const height = (bar.value / high) * INNER_HEIGHT;
            return (
              <rect
                key={bar.key}
                x={centreOf(index) - width / 2}
                y={PADDING.top + INNER_HEIGHT - height}
                width={width}
                height={height}
                rx={Math.min(2, width / 2)}
                fill={color}
                fillOpacity={
                  hover.index === null || hover.index === index ? 1 : DIM
                }
              />
            );
          })}
          {labelledTicks(bars, (bar) => bar.label, slot, MAX_TICKS).map(
            (tick) => (
              <text
                key={tick.entry.key}
                x={tickX(centreOf(tick.index), tick.entry.label)}
                y={TICK_BASELINE}
                textAnchor="middle"
                fontSize={TICK_FONT}
                fill="currentColor"
                fillOpacity={0.6}
              >
                {tick.entry.label}
              </text>
            ),
          )}
        </svg>
        {hovered !== null && hover.index !== null && (
          <ChartTooltip
            at={centreOf(hover.index)}
            label={hovered.label}
            readings={[{ key: "value", color, text: format(hovered.value) }]}
          />
        )}
      </div>
    </div>
  );
}
