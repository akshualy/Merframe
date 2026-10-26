import type { EventName } from "@/lib/bridge";
import { fullDate } from "@/lib/format";
import type {
  ArbitrationNode,
  ArbitrationSchedule,
  ArbitrationSlot,
  ArbitrationTier,
} from "@/types";

export const ARBITRATION_RELOAD_EVENTS: EventName[] = ["worldStateUpdated"];

export const ARBITRATION_TIERS: ArbitrationTier[] = [
  "S",
  "A",
  "B",
  "C",
  "D",
  "F",
];

export function slotPlace(slot: {
  node: string;
  planet: string | null;
}): string {
  return slot.planet ? `${slot.node}, ${slot.planet}` : slot.node;
}

export function slotMission(slot: { mission_type: string | null }): string {
  return slot.mission_type ?? "Unknown mission";
}

export function nodeValues(
  nodes: ArbitrationNode[],
  key: "mission_type" | "planet" | "faction",
): string[] {
  const known = nodes
    .map((node) => node[key])
    .filter((value) => value !== null);
  return [...new Set(known)].sort((a, b) => a.localeCompare(b));
}

function startsAt(slot: ArbitrationSlot): number {
  return new Date(slot.starts).getTime();
}

function endsAt(slot: ArbitrationSlot): number {
  return new Date(slot.ends).getTime();
}

export function liveSlots(
  schedule: ArbitrationSchedule | null,
  now: number,
): ArbitrationSlot[] {
  return (schedule?.slots ?? []).filter((slot) => endsAt(slot) > now);
}

export function currentSlot(
  slots: ArbitrationSlot[],
  now: number,
): ArbitrationSlot | null {
  return (
    slots.find((slot) => startsAt(slot) <= now && endsAt(slot) > now) ?? null
  );
}

export function nextSlot(
  slots: ArbitrationSlot[],
  now: number,
): ArbitrationSlot | null {
  return slots.find((slot) => startsAt(slot) > now) ?? null;
}

export function slotSearchText(slot: ArbitrationSlot): string {
  return [slot.node, slot.planet, slot.mission_type, slot.faction, slot.tier]
    .filter((part) => part !== null)
    .join(" ")
    .toLocaleLowerCase();
}

export function dayKey(at: number): string {
  const date = new Date(at);
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${date.getFullYear()}-${month}-${day}`;
}

function tomorrowKey(now: number): string {
  const tomorrow = new Date(now);
  tomorrow.setDate(tomorrow.getDate() + 1);
  return dayKey(tomorrow.getTime());
}

export function dayName(at: number, now: number): string {
  const key = dayKey(at);
  if (key === dayKey(now)) {
    return "Today";
  }

  if (key === tomorrowKey(now)) {
    return "Tomorrow";
  }

  return new Date(at).toLocaleDateString("en-GB", { weekday: "short" });
}

export interface ArbitrationDay {
  key: string;
  label: string;
  date: string;
  slots: ArbitrationSlot[];
}

export function groupByDay(
  slots: ArbitrationSlot[],
  now: number,
): ArbitrationDay[] {
  const days: ArbitrationDay[] = [];
  for (const slot of slots) {
    const at = startsAt(slot);
    const key = dayKey(at);
    const day = days.find((day) => day.key === key);
    if (day) {
      day.slots.push(slot);
    } else {
      days.push({
        key,
        label: dayName(at, now),
        date: fullDate(slot.starts),
        slots: [slot],
      });
    }
  }

  return days;
}
