import type { ReactNode } from "react";
import {
  Cell,
  PieChart as Chart,
  Pie,
  ResponsiveContainer,
  Tooltip,
} from "recharts";
import { ChartReadings } from "@/components/chart-hover";
import { Hint } from "@/components/ui/hint";
import { percent } from "@/lib/format";

export interface Slice {
  key: string;
  label: string;
  color: string;
  value: number;
}

export function PieChart({
  label,
  slices,
  format,
  action,
}: {
  label: string;
  slices: Slice[];
  format: (value: number) => string;
  action?: ReactNode;
}) {
  const shown = slices.filter((slice) => slice.value > 0);
  const total = shown.reduce((sum, slice) => sum + slice.value, 0);

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <Hint as="span">{label}</Hint>
        {action}
      </div>
      <div className="flex flex-wrap items-center gap-6">
        <div className="size-48 shrink-0">
          <ResponsiveContainer>
            <Chart>
              <Pie
                data={shown}
                dataKey="value"
                nameKey="label"
                startAngle={90}
                endAngle={-270}
                stroke="var(--card)"
                strokeWidth={2}
                isAnimationActive={false}
              >
                {shown.map((slice) => (
                  <Cell key={slice.key} fill={slice.color} />
                ))}
              </Pie>
              <Tooltip
                isAnimationActive={false}
                content={({ active, payload }) => {
                  const slice: Slice | undefined = payload[0]?.payload;
                  return active && slice ? (
                    <ChartReadings
                      label={slice.label}
                      readings={[
                        {
                          key: slice.key,
                          color: slice.color,
                          text: format(slice.value),
                          note: percent(slice.value / total, 0),
                        },
                      ]}
                    />
                  ) : null;
                }}
              />
            </Chart>
          </ResponsiveContainer>
        </div>
        <ul className="flex min-w-48 flex-1 flex-col gap-1 text-sm">
          {shown.map((slice) => (
            <li key={slice.key} className="flex items-center gap-2">
              <span
                className="size-2.5 shrink-0 rounded-sm"
                style={{ background: slice.color }}
                aria-hidden="true"
              />
              <span className="text-foreground flex-1 truncate">
                {slice.label}
              </span>
              <span className="text-foreground font-bold tabular-nums">
                {format(slice.value)}
              </span>
              <span className="text-muted-foreground w-10 text-right text-xs tabular-nums">
                {percent(slice.value / total, 0)}
              </span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
