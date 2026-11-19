import { SelectField } from "@/components/option-select";
import type { OverlayMonitor, OverlayMonitorOrigin } from "@/types";

const AUTOMATIC = "automatic";

function monitorKey(origin: OverlayMonitorOrigin) {
  return `${origin.x},${origin.y}`;
}

function monitorWithKey(monitors: OverlayMonitor[], key: string) {
  return monitors.find((monitor) => monitorKey(monitor.origin) === key);
}

function monitorOptions(
  monitors: OverlayMonitor[],
  chosen: OverlayMonitorOrigin | null,
) {
  const options = [
    { value: AUTOMATIC, label: "Automatic" },
    ...monitors.map((monitor, index) => ({
      value: monitorKey(monitor.origin),
      label: `Monitor ${index + 1}, ${monitor.width}x${monitor.height}${monitor.primary ? " (primary)" : ""}`,
    })),
  ];
  const unplugged = chosen && !monitors.some((monitor) => monitor.chosen);
  return unplugged
    ? [
        ...options,
        {
          value: monitorKey(chosen),
          label: `Monitor at ${chosen.x}, ${chosen.y} (not connected)`,
        },
      ]
    : options;
}

export function OverlayMonitorSelect({
  monitors,
  chosen,
  disabled,
  onChange,
}: {
  monitors: OverlayMonitor[];
  chosen: OverlayMonitorOrigin | null;
  disabled: boolean;
  onChange: (chosen: OverlayMonitorOrigin | null) => void;
}) {
  return (
    <SelectField
      id="overlay-monitor"
      label="Monitor"
      inline
      value={chosen ? monitorKey(chosen) : AUTOMATIC}
      options={monitorOptions(monitors, chosen)}
      disabled={disabled}
      onChange={(value) =>
        onChange(monitorWithKey(monitors, value)?.origin ?? null)
      }
      size="sm"
      className="w-64"
    />
  );
}
