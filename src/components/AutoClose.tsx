import { useEffect, useId, useState, type ReactElement } from "react";
import { TIMEOUT_MAX_SECONDS, TIMEOUT_MIN_SECONDS } from "../lib/api";
import { Toggle } from "./Toggle";

interface AutoCloseProps {
  enabled: boolean;
  seconds: number;
  onSave: (enabled: boolean, seconds: number) => Promise<boolean>;
}

export function AutoClose({ enabled, seconds, onSave }: AutoCloseProps): ReactElement {
  const sliderId = useId();
  const [draft, setDraft] = useState(seconds);
  const [saving, setSaving] = useState(false);

  useEffect(() => setDraft(seconds), [seconds]);

  const save = async (nextEnabled: boolean, nextSeconds: number): Promise<void> => {
    setSaving(true);
    const saved = await onSave(nextEnabled, nextSeconds);
    if (!saved) setDraft(seconds);
    setSaving(false);
  };

  const settle = (value: number): void => {
    if (value !== seconds) void save(enabled, value);
  };

  return (
    <div className="flex flex-col gap-2 px-3 py-2.5">
      <div className="flex items-center gap-3">
        <label htmlFor={sliderId} className="min-w-0 flex-1">
          <span className="block text-[13.5px] font-medium text-ink">Close automatically after</span>
          <span className="block text-[12px] text-ink-3">
            {enabled ? "Applies from the next link you open." : "The window stays until you choose or press Esc."}
          </span>
        </label>
        <span
          aria-hidden="true"
          className={`w-10 shrink-0 text-right text-[13px] tabular-nums ${enabled ? "text-ink" : "text-ink-3"}`}
        >
          {draft} s
        </span>
        <label className="flex shrink-0 items-center gap-2 border-l border-line pl-3">
          <span className="text-[12.5px] text-ink-2">Never</span>
          <Toggle
            checked={!enabled}
            label="Never close automatically"
            disabled={saving}
            onChange={(never) => void save(!never, draft)}
          />
        </label>
      </div>
      <input
        id={sliderId}
        type="range"
        min={TIMEOUT_MIN_SECONDS}
        max={TIMEOUT_MAX_SECONDS}
        step={1}
        value={draft}
        disabled={!enabled}
        aria-valuetext={`${draft} seconds`}
        onChange={(event) => setDraft(Number(event.currentTarget.value))}
        onPointerUp={(event) => settle(Number(event.currentTarget.value))}
        onKeyUp={(event) => settle(Number(event.currentTarget.value))}
        onBlur={(event) => settle(Number(event.currentTarget.value))}
        className="h-4 w-full cursor-pointer accent-accent outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent disabled:cursor-default disabled:opacity-40"
      />
      <div className="flex justify-between text-[11.5px] text-ink-3 tabular-nums" aria-hidden="true">
        <span>{TIMEOUT_MIN_SECONDS} s</span>
        <span>{TIMEOUT_MAX_SECONDS} s</span>
      </div>
    </div>
  );
}
