import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  LineChart as Chart,
  Line,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { ChartHeader, readingsTooltip } from "@/components/chart-hover";
import {
  AXIS_TICK,
  BAR_CURSOR,
  COMPACT,
  changeColor,
  changeTone,
  GRID,
  LINE_CURSOR,
  MARGIN,
  signed,
} from "@/lib/chart";
import { cn } from "@/lib/utils";

const MAX_DOTS = 12;
const AXIS_WIDTH = 46;

export interface Point {
  key: string;
  label: string;
  value: number | null;
  change: number | null;
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
  const known = points.filter(
    (point): point is Point & { value: number } => point.value !== null,
  );
  const first = known.at(0);
  const latest = known.at(-1);
  const swing = Math.max(
    1,
    ...points.map((point) => Math.abs(point.change ?? 0)),
  );
  const dot = { r: 3, fill: color, stroke: "var(--card)", strokeWidth: 2 };

  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <ChartHeader
        color={color}
        label={label}
        note={
          first && latest ? (
            <span className={changeTone(latest.value - first.value)}>
              {signed(latest.value - first.value, format)} over the window
            </span>
          ) : null
        }
        value={latest ? format(latest.value) : "-"}
        caption={latest?.label ?? "no snapshots in this window"}
      />
      <div className="h-28">
        <ResponsiveContainer>
          <Chart data={points} syncId={label} margin={MARGIN}>
            <CartesianGrid vertical={false} {...GRID} />
            <XAxis dataKey="label" hide />
            <YAxis
              width={AXIS_WIDTH}
              domain={["dataMin", "dataMax"]}
              padding={{ top: 4, bottom: 4 }}
              tickCount={2}
              tick={AXIS_TICK}
              tickFormatter={(value: number) => COMPACT.format(value)}
              tickLine={false}
              axisLine={false}
            />
            <Tooltip
              cursor={LINE_CURSOR}
              isAnimationActive={false}
              content={readingsTooltip<Point>((point) => [
                {
                  key: "value",
                  color,
                  text: point.value === null ? "-" : format(point.value),
                },
                ...(point.change === null
                  ? []
                  : [
                      {
                        key: "change",
                        color: changeColor(point.change),
                        text: signed(point.change, format),
                        note: "change",
                      },
                    ]),
              ])}
            />
            <Line
              dataKey="value"
              type="monotone"
              stroke={color}
              strokeWidth={2}
              connectNulls
              dot={points.length <= MAX_DOTS ? dot : false}
              activeDot={{ ...dot, r: 4 }}
              isAnimationActive={false}
            />
          </Chart>
        </ResponsiveContainer>
      </div>
      <div className="h-12">
        <ResponsiveContainer>
          <BarChart data={points} syncId={label} margin={MARGIN}>
            <XAxis
              dataKey="label"
              tick={AXIS_TICK}
              tickLine={false}
              axisLine={false}
              minTickGap={24}
            />
            <YAxis
              width={AXIS_WIDTH}
              domain={[-swing, swing]}
              ticks={[0]}
              tick={AXIS_TICK}
              tickFormatter={() => "change"}
              tickLine={false}
              axisLine={false}
            />
            <ReferenceLine y={0} stroke="currentColor" strokeOpacity={0.2} />
            <Tooltip cursor={BAR_CURSOR} content={() => null} />
            <Bar dataKey="change" maxBarSize={8} isAnimationActive={false}>
              {points.map((point) => (
                <Cell key={point.key} fill={changeColor(point.change ?? 0)} />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
