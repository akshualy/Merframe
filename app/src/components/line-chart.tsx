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

const LANE_HEIGHT = 26;
const LANE_GAP = 10;
const PLOT_HEIGHT = INNER_HEIGHT - LANE_HEIGHT - LANE_GAP;
const LANE_TOP = PADDING.top + PLOT_HEIGHT + LANE_GAP;
const LANE_ZERO = LANE_TOP + LANE_HEIGHT / 2;
const MAX_TICKS = 4;
const MAX_DOTS = 12;
const SMOOTHING = 0.6;
const MAX_BAR_WIDTH = 8;
const DIM = 0.35;

export interface Point {
  key: string;
  label: string;
  value: number | null;
  change: number | null;
}

interface Scale {
  low: number;
  high: number;
}

function scaleOf(points: Point[]): Scale {
  const known = points
    .map((point) => point.value)
    .filter((value): value is number => value !== null);
  if (known.length === 0) {
    return { low: 0, high: 1 };
  }
  const low = Math.min(...known);
  const high = Math.max(...known);
  if (low === high) {
    return { low: low - 1, high: high + 1 };
  }
  return { low, high };
}

function xOf(index: number, count: number) {
  if (count < 2) {
    return PADDING.left + INNER_WIDTH / 2;
  }
  return PADDING.left + (index * INNER_WIDTH) / (count - 1);
}

function yOf(value: number, scale: Scale) {
  const span = scale.high - scale.low;
  return PADDING.top + PLOT_HEIGHT - ((value - scale.low) / span) * PLOT_HEIGHT;
}

function fixed(value: number) {
  return value.toFixed(1);
}

interface Vertex {
  key: string;
  x: number;
  y: number;
}

function knownAt(points: Point[], scale: Scale): Vertex[] {
  return points.flatMap((point, index) =>
    point.value === null
      ? []
      : [
          {
            key: point.key,
            x: xOf(index, points.length),
            y: yOf(point.value, scale),
          },
        ],
  );
}

function path(known: Vertex[]) {
  return known.reduce((out, end, index) => {
    if (index === 0) {
      return `M${fixed(end.x)} ${fixed(end.y)}`;
    }
    const start = known[index - 1] ?? end;
    const before = known[index - 2] ?? start;
    const after = known[index + 1] ?? end;
    const c1x = start.x + ((end.x - before.x) / 6) * SMOOTHING;
    const c1y = start.y + ((end.y - before.y) / 6) * SMOOTHING;
    const c2x = end.x - ((after.x - start.x) / 6) * SMOOTHING;
    const c2y = end.y - ((after.y - start.y) / 6) * SMOOTHING;
    return `${out} C${fixed(c1x)} ${fixed(c1y)} ${fixed(c2x)} ${fixed(c2y)} ${fixed(end.x)} ${fixed(end.y)}`;
  }, "");
}

function signed(value: number, format: (value: number) => string) {
  return `${value > 0 ? "+" : ""}${format(value)}`;
}

function net(points: Point[]) {
  const known = points
    .map((point) => point.value)
    .filter((value): value is number => value !== null);
  const first = known.at(0);
  const last = known.at(-1);
  if (first === undefined || last === undefined) {
    return null;
  }
  return last - first;
}

export function LineChart({
  label,
  color,
  points,
  format,
  className,
}: {
  label: string;
  color: string;
  points: Point[];
  format: (value: number) => string;
  className?: string;
}) {
  const scale = scaleOf(points);
  const vertices = knownAt(points, scale);
  const delta = net(points);
  const known = points.filter((point) => point.value !== null);
  const latest = known.at(-1)?.value ?? null;
  const dots = points.length <= MAX_DOTS;
  const swing = Math.max(
    1,
    ...points.map((point) => Math.abs(point.change ?? 0)),
  );
  const barWidth = Math.min(
    MAX_BAR_WIDTH,
    Math.max(1, (INNER_WIDTH / Math.max(1, points.length)) * 0.5),
  );
  const pitch = INNER_WIDTH / Math.max(1, points.length - 1);
  const hover = useChartHover(points.length, (x) =>
    Math.min(
      points.length - 1,
      Math.max(0, Math.round((x - PADDING.left) / pitch)),
    ),
  );
  const hovered = hover.index === null ? null : (points[hover.index] ?? null);

  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <ChartHeader
        color={color}
        label={label}
        note={
          delta === null ? null : `${signed(delta, format)} over the window`
        }
        value={latest === null ? "-" : format(latest)}
        caption={known.at(-1)?.label ?? "no snapshots in this window"}
      />
      <div className="relative">
        <svg
          viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
          className="w-full"
          role="img"
          aria-label={`${label} per day, with the change from the day before`}
          {...hover.handlers}
        >
          <title>{label}</title>
          {[0, 0.5, 1].map((fraction) => (
            <line
              key={fraction}
              x1={PADDING.left}
              x2={WIDTH - PADDING.right}
              y1={PADDING.top + PLOT_HEIGHT * fraction}
              y2={PADDING.top + PLOT_HEIGHT * fraction}
              stroke="currentColor"
              strokeOpacity={0.1}
            />
          ))}
          {[scale.high, scale.low].map((value) => (
            <text
              key={value}
              x={PADDING.left - 6}
              y={yOf(value, scale) + 3}
              textAnchor="end"
              fontSize={TICK_FONT}
              fill="currentColor"
              fillOpacity={0.6}
            >
              {COMPACT.format(value)}
            </text>
          ))}
          {hover.index !== null && (
            <line
              x1={xOf(hover.index, points.length)}
              x2={xOf(hover.index, points.length)}
              y1={PADDING.top}
              y2={PADDING.top + INNER_HEIGHT}
              stroke="currentColor"
              strokeOpacity={0.3}
            />
          )}
          <path
            d={path(vertices)}
            fill="none"
            stroke={color}
            strokeWidth={2}
            strokeLinecap="round"
            strokeLinejoin="round"
          />
          {vertices
            .filter((_, index) => dots || index === vertices.length - 1)
            .map((entry, index, drawn) => (
              <circle
                key={entry.key}
                cx={entry.x}
                cy={entry.y}
                r={index === drawn.length - 1 ? 4 : 3}
                fill={color}
                stroke="var(--card)"
                strokeWidth={2}
              />
            ))}
          {hovered !== null &&
            hovered.value !== null &&
            hover.index !== null && (
              <circle
                cx={xOf(hover.index, points.length)}
                cy={yOf(hovered.value, scale)}
                r={4}
                fill={color}
                stroke="var(--card)"
                strokeWidth={2}
              />
            )}
          <text
            x={PADDING.left - 6}
            y={LANE_ZERO + 3}
            textAnchor="end"
            fontSize={TICK_FONT}
            fill="currentColor"
            fillOpacity={0.6}
          >
            change
          </text>
          <line
            x1={PADDING.left}
            x2={WIDTH - PADDING.right}
            y1={LANE_ZERO}
            y2={LANE_ZERO}
            stroke="currentColor"
            strokeOpacity={0.2}
          />
          {points.map((point, index) => {
            const height =
              point.change === null || point.change === 0
                ? 0
                : (Math.abs(point.change) / swing) * (LANE_HEIGHT / 2);
            if (height === 0) {
              return null;
            }
            const up = (point.change ?? 0) > 0;
            return (
              <rect
                key={point.key}
                x={xOf(index, points.length) - barWidth / 2}
                y={up ? LANE_ZERO - height : LANE_ZERO}
                width={barWidth}
                height={height}
                fill={up ? "var(--chart-gain)" : "var(--chart-loss)"}
                fillOpacity={
                  hover.index === null || hover.index === index ? 1 : DIM
                }
              />
            );
          })}
          {labelledTicks(points, (point) => point.label, pitch, MAX_TICKS).map(
            (tick) => (
              <text
                key={tick.entry.key}
                x={tickX(xOf(tick.index, points.length), tick.entry.label)}
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
            at={xOf(hover.index, points.length)}
            label={hovered.label}
            readings={[
              {
                key: "value",
                color,
                text: hovered.value === null ? "-" : format(hovered.value),
              },
              ...(hovered.change === null
                ? []
                : [
                    {
                      key: "change",
                      color:
                        hovered.change < 0
                          ? "var(--chart-loss)"
                          : "var(--chart-gain)",
                      text: signed(hovered.change, format),
                      note: "change",
                    },
                  ]),
            ]}
          />
        )}
      </div>
    </div>
  );
}
