import {
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
  type KeyboardEvent,
  type PointerEvent,
  type ReactElement,
} from "react";
import type { BrowserView } from "../lib/api";
import { clampOffset, layoutSlots, moveItem, shiftFor, slotTop, targetIndex, type Slot } from "../lib/reorder";
import { GlobeIcon, GripIcon } from "./Icons";
import { Toggle } from "./Toggle";

interface BrowserListProps {
  browsers: BrowserView[];
  savingId: string | null;
  locked: boolean;
  onToggle: (browser: BrowserView, visible: boolean) => void;
  onReorder: (next: BrowserView[]) => void;
}

interface Drag {
  id: string;
  from: number;
  to: number;
  offset: number;
  slots: Slot[];
}

interface Session {
  id: string;
  from: number;
  pointerId: number;
  slots: Slot[];
  startY: number;
  startScroll: number;
  pointerY: number;
  scroller: HTMLElement | null;
}

const AUTOSCROLL_EDGE = 40;

function resolve(session: Session): Drag {
  const scrolled = session.scroller ? session.scroller.scrollTop - session.startScroll : 0;
  const offset = clampOffset(session.slots, session.from, session.pointerY - session.startY + scrolled);
  return {
    id: session.id,
    from: session.from,
    to: targetIndex(session.slots, session.from, offset),
    offset,
    slots: session.slots,
  };
}

export function BrowserList({ browsers, savingId, locked, onToggle, onReorder }: BrowserListProps): ReactElement {
  const listRef = useRef<HTMLDivElement>(null);
  const rows = useRef(new Map<string, HTMLDivElement>());
  const handles = useRef(new Map<string, HTMLButtonElement>());
  const session = useRef<Session | null>(null);
  const [drag, setDrag] = useState<Drag | null>(null);
  const [announcement, setAnnouncement] = useState("");
  const [focusId, setFocusId] = useState<string | null>(null);
  const idPrefix = useId();
  const dragging = drag !== null;

  const commit = useCallback(
    (from: number, to: number): void => {
      const moved = browsers[from];
      onReorder(moveItem(browsers, from, to));
      setFocusId(moved.id);
      setAnnouncement(`${moved.name} moved to position ${to + 1} of ${browsers.length}.`);
    },
    [browsers, onReorder],
  );

  const finish = useCallback(
    (apply: boolean): void => {
      const current = session.current;
      if (!current) return;
      session.current = null;
      const handle = handles.current.get(current.id);
      if (handle?.hasPointerCapture(current.pointerId)) handle.releasePointerCapture(current.pointerId);
      const result = resolve(current);
      setDrag(null);
      if (apply && result.to !== result.from) commit(result.from, result.to);
      else setFocusId(current.id);
    },
    [commit],
  );

  const begin = (event: PointerEvent<HTMLButtonElement>, browser: BrowserView, index: number): void => {
    if (event.button !== 0 || session.current || locked) return;
    const slots = layoutSlots(browsers.map((item) => rows.current.get(item.id)?.getBoundingClientRect().height ?? 0));
    const scroller = listRef.current?.closest<HTMLElement>("main") ?? null;
    event.currentTarget.setPointerCapture(event.pointerId);
    session.current = {
      id: browser.id,
      from: index,
      pointerId: event.pointerId,
      slots,
      startY: event.clientY,
      startScroll: scroller?.scrollTop ?? 0,
      pointerY: event.clientY,
      scroller,
    };
    setDrag(resolve(session.current));
  };

  const track = (event: PointerEvent<HTMLButtonElement>): void => {
    const current = session.current;
    if (!current || current.pointerId !== event.pointerId) return;
    current.pointerY = event.clientY;
    setDrag(resolve(current));
  };

  const onHandleKey = (event: KeyboardEvent<HTMLButtonElement>, index: number): void => {
    if (!event.altKey || (event.key !== "ArrowUp" && event.key !== "ArrowDown")) return;
    event.preventDefault();
    event.stopPropagation();
    if (locked) return;
    const to = index + (event.key === "ArrowUp" ? -1 : 1);
    if (to < 0 || to >= browsers.length) return;
    commit(index, to);
  };

  useEffect(() => {
    if (!dragging) return;
    const onKey = (event: globalThis.KeyboardEvent): void => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopImmediatePropagation();
      finish(false);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [dragging, finish]);

  useEffect(() => {
    if (!dragging) return;
    let frame = 0;
    const tick = (): void => {
      const current = session.current;
      if (current?.scroller) {
        const bounds = current.scroller.getBoundingClientRect();
        let speed = 0;
        if (current.pointerY < bounds.top + AUTOSCROLL_EDGE) {
          speed = -Math.ceil((bounds.top + AUTOSCROLL_EDGE - current.pointerY) / 5);
        } else if (current.pointerY > bounds.bottom - AUTOSCROLL_EDGE) {
          speed = Math.ceil((current.pointerY - (bounds.bottom - AUTOSCROLL_EDGE)) / 5);
        }
        if (speed !== 0) {
          current.scroller.scrollTop += speed;
          setDrag(resolve(current));
        }
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [dragging]);

  useEffect(() => {
    if (focusId === null) return;
    handles.current.get(focusId)?.focus();
    setFocusId(null);
  }, [focusId, browsers]);

  if (browsers.length === 0) return <p className="px-3 py-3 text-[13px] text-ink-2">No browsers found.</p>;

  return (
    <>
      <div ref={listRef} role="list" className="relative flex flex-col">
        {drag && (
          <div
            aria-hidden="true"
            className="pointer-events-none absolute inset-x-1.5 rounded-md border border-dashed border-accent/70 bg-accent/10"
            style={{
              top: slotTop(drag.slots, drag.from, drag.to) + 3,
              height: drag.slots[drag.from].height - 6,
            }}
          />
        )}
        {browsers.map((browser, index) => {
          const lifted = drag?.id === browser.id;
          const shift = drag ? (lifted ? drag.offset : shiftFor(drag.slots, drag.from, drag.to, index)) : 0;
          const toggleId = `${idPrefix}-${index}`;
          return (
            <div
              key={browser.id}
              role="listitem"
              ref={(element) => {
                if (element) rows.current.set(browser.id, element);
                else rows.current.delete(browser.id);
              }}
              style={shift ? { transform: `translateY(${shift}px)` } : undefined}
              className={`flex items-center gap-2 py-2.5 pr-3 pl-1.5 ${
                lifted
                  ? "relative z-10 rounded-lg border-t border-transparent bg-hover shadow-lg ring-1 ring-line"
                  : `${index > 0 ? "border-t border-line" : ""} ${
                      dragging ? "transition-transform duration-200 ease-out-quart motion-reduce:transition-none" : ""
                    }`
              }`}
            >
              <button
                type="button"
                ref={(element) => {
                  if (element) handles.current.set(browser.id, element);
                  else handles.current.delete(browser.id);
                }}
                aria-label={`Reorder ${browser.name}`}
                aria-disabled={locked}
                aria-keyshortcuts="Alt+ArrowUp Alt+ArrowDown"
                title="Drag to reorder, or press Alt+Up or Alt+Down"
                onPointerDown={(event) => begin(event, browser, index)}
                onPointerMove={track}
                onPointerUp={() => finish(true)}
                onPointerCancel={() => finish(false)}
                onKeyDown={(event) => onHandleKey(event, index)}
                className={`grid h-8 w-6 shrink-0 touch-none place-items-center rounded-md outline-none transition-colors duration-150 hover:bg-hover hover:text-ink-2 focus-visible:outline-2 focus-visible:outline-accent ${
                  lifted ? "cursor-grabbing text-ink-2" : "cursor-grab text-ink-3"
                } ${locked ? "opacity-40" : ""}`}
              >
                <GripIcon />
              </button>
              <label htmlFor={toggleId} className="flex min-w-0 flex-1 items-center gap-3">
                <span className="grid h-5 w-5 shrink-0 place-items-center text-ink-2">
                  {browser.icon ? (
                    <img src={browser.icon} alt="" className="h-5 w-5" draggable={false} />
                  ) : (
                    <GlobeIcon />
                  )}
                </span>
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-[13.5px] font-medium text-ink">{browser.name}</span>
                  <span className="block truncate font-mono text-[11px] text-ink-3" title={browser.path}>
                    {browser.path}
                  </span>
                </span>
              </label>
              <Toggle
                id={toggleId}
                checked={!browser.hidden}
                label={`Show ${browser.name}`}
                disabled={savingId === browser.id}
                onChange={(visible) => onToggle(browser, visible)}
              />
            </div>
          );
        })}
      </div>
      <p role="status" aria-live="polite" className="sr-only">
        {announcement}
      </p>
    </>
  );
}
