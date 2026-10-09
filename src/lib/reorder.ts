export function moveItem<T>(items: readonly T[], from: number, to: number): T[] {
  const next = items.slice();
  const [moved] = next.splice(from, 1);
  next.splice(to, 0, moved);
  return next;
}

const EDGE_TOLERANCE = 0.5;

export interface Slot {
  top: number;
  height: number;
}

export function layoutSlots(heights: readonly number[]): Slot[] {
  let top = 0;
  return heights.map((height) => {
    const slot = { top, height };
    top += height;
    return slot;
  });
}

export function targetIndex(slots: readonly Slot[], from: number, offset: number): number {
  const dragged = slots[from];
  const last = slots[slots.length - 1];
  if (offset <= -dragged.top + EDGE_TOLERANCE) return 0;
  if (offset >= last.top + last.height - dragged.top - dragged.height - EDGE_TOLERANCE) return slots.length - 1;
  const centre = dragged.top + dragged.height / 2 + offset;
  let target = 0;
  slots.forEach((slot, index) => {
    if (index !== from && slot.top + slot.height / 2 < centre) target += 1;
  });
  return target;
}

export function clampOffset(slots: readonly Slot[], from: number, offset: number): number {
  const dragged = slots[from];
  const last = slots[slots.length - 1];
  const lowest = -dragged.top;
  const highest = last.top + last.height - dragged.top - dragged.height;
  return Math.min(Math.max(offset, lowest), highest);
}

export function shiftFor(slots: readonly Slot[], from: number, to: number, index: number): number {
  const height = slots[from].height;
  if (index === from) return 0;
  if (from < to && index > from && index <= to) return -height;
  if (from > to && index >= to && index < from) return height;
  return 0;
}

export function slotTop(slots: readonly Slot[], from: number, to: number): number {
  const order = moveItem(slots.map((_, index) => index), from, to);
  let top = 0;
  for (const index of order) {
    if (index === from) return top;
    top += slots[index].height;
  }
  return top;
}
