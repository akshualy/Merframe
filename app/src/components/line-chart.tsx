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
import { ChartHeader, ChartReadings } from "@/components/chart-hover";
import { AXIS_TICK, COMPACT, changeTone, GRID } from "@/lib/chart";
import { cn } from "@/lib/utils";

const MAX_DOTS = 12;
const AXIS_WIDTH = 46;
const MARGIN = { top: 8, right: 8, bottom: 0, left: 0 };
const CURSOR = { stroke: "currentColor", strokeOpacity: 0.3 };

export interface Point {
  key: string;
  label: string;
  value: number | null;
  change: number | null;
}

function signed(value: number, format: (value: number) => string) {
  return `${value > 0 ? "+" : ""}${format(value)}`;
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
              tickCount={2}
              tick={AXIS_TICK}
              tickFormatter={(value: number) => COMPACT.format(value)}
              tickLine={false}
              axisLine={false}
            />
            <Tooltip
              cursor={CURSOR}
              isAnimationActive={false}
              content={({ active, payload }) => {
                const point: Point | undefined = payload[0]?.payload;
                return active && point ? (
                  <ChartReadings
                    label={point.label}
                    readings={[
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
                              color:
                                point.change < 0
                                  ? "var(--chart-loss)"
                                  : "var(--chart-gain)",
                              text: signed(point.change, format),
                              note: "change",
                            },
                          ]),
                    ]}
                  />
                ) : null;
              }}
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
            <Tooltip cursor={CURSOR} content={() => null} />
            <Bar dataKey="change" maxBarSize={8} isAnimationActive={false}>
              {points.map((point) => (
                <Cell
                  key={point.key}
                  fill={
                    (point.change ?? 0) < 0
                      ? "var(--chart-loss)"
                      : "var(--chart-gain)"
                  }
                />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
