import {
  Bar as BarMark,
  CartesianGrid,
  BarChart as Chart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { ChartHeader, ChartReadings } from "@/components/chart-hover";
import { AXIS_TICK, COMPACT, GRID } from "@/lib/chart";
import { cn } from "@/lib/utils";

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
  const total = bars.reduce((sum, bar) => sum + bar.value, 0);

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
      <div className="h-40">
        <ResponsiveContainer>
          <Chart data={bars} margin={{ top: 8, right: 8, bottom: 0, left: 0 }}>
            <CartesianGrid vertical={false} {...GRID} />
            <XAxis
              dataKey="label"
              tick={AXIS_TICK}
              tickLine={false}
              axisLine={false}
              minTickGap={24}
            />
            <YAxis
              width={40}
              domain={[0, high]}
              ticks={[0, high]}
              tick={AXIS_TICK}
              tickFormatter={(value: number) => COMPACT.format(value)}
              tickLine={false}
              axisLine={false}
            />
            <Tooltip
              cursor={{ fill: "currentColor", fillOpacity: 0.08 }}
              isAnimationActive={false}
              content={({ active, payload }) => {
                const bar: Bar | undefined = payload[0]?.payload;
                return active && bar ? (
                  <ChartReadings
                    label={bar.label}
                    readings={[
                      { key: "value", color, text: format(bar.value) },
                    ]}
                  />
                ) : null;
              }}
            />
            <BarMark
              dataKey="value"
              fill={color}
              radius={[4, 4, 0, 0]}
              maxBarSize={24}
              activeBar={{ fillOpacity: 1 }}
              isAnimationActive={false}
            />
          </Chart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
