import {
  Bar,
  BarChart,
  Cell,
  LabelList,
  type LabelProps,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { ChartReadings } from "@/components/chart-hover";
import { Hint } from "@/components/ui/hint";
import { AXIS_TICK } from "@/lib/chart";

export interface ChangeRow {
  key: string;
  label: string;
  change: number;
  note: string;
}

export function signedPercent(change: number): string {
  const percent = Math.round(change * 100);
  return `${percent > 0 ? "+" : ""}${percent}%`;
}

function EndLabel({ x, y, width, height, value }: LabelProps) {
  const change = Number(value);
  const left = Number(x);
  const span = Number(width);
  const anchor = change < 0 ? left - 4 : left + span + 4;
  return (
    <text
      x={anchor}
      y={Number(y) + Number(height) / 2}
      dominantBaseline="central"
      textAnchor={change < 0 ? "end" : "start"}
      {...AXIS_TICK}
      fontSize={12}
    >
      {signedPercent(change)}
    </text>
  );
}

function colorOf(change: number): string {
  return change < 0 ? "var(--chart-loss)" : "var(--chart-gain)";
}

export function ChangeBars({
  label,
  rows,
  empty,
}: {
  label: string;
  rows: ChangeRow[];
  empty: string;
}) {
  const reach =
    1.4 * Math.max(0.01, ...rows.map((row) => Math.abs(row.change)));

  return (
    <div className="flex flex-col gap-3">
      <div className="flex min-h-10 items-center">
        <Hint as="span">{label}</Hint>
      </div>
      {rows.length === 0 ? (
        <Hint>{empty}</Hint>
      ) : (
        <div className="h-48">
          <ResponsiveContainer>
            <BarChart
              data={rows}
              layout="vertical"
              margin={{ top: 0, right: 0, bottom: 0, left: 0 }}
            >
              <XAxis type="number" domain={[-reach, reach]} hide />
              <YAxis
                type="category"
                dataKey="label"
                width={84}
                tick={{ ...AXIS_TICK, fontSize: 12 }}
                tickLine={false}
                axisLine={false}
              />
              <ReferenceLine x={0} stroke="currentColor" strokeOpacity={0.2} />
              <Tooltip
                cursor={{ fill: "currentColor", fillOpacity: 0.08 }}
                isAnimationActive={false}
                content={({ active, payload }) => {
                  const row: ChangeRow | undefined = payload[0]?.payload;
                  return active && row ? (
                    <ChartReadings
                      label={row.label}
                      readings={[
                        {
                          key: row.key,
                          color: colorOf(row.change),
                          text: signedPercent(row.change),
                          note: row.note,
                        },
                      ]}
                    />
                  ) : null;
                }}
              />
              <Bar
                dataKey="change"
                radius={4}
                maxBarSize={12}
                isAnimationActive={false}
              >
                {rows.map((row) => (
                  <Cell key={row.key} fill={colorOf(row.change)} />
                ))}
                <LabelList dataKey="change" content={EndLabel} />
              </Bar>
            </BarChart>
          </ResponsiveContainer>
        </div>
      )}
    </div>
  );
}
