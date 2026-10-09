import { useState, type ReactElement, type ReactNode } from "react";
import { api, errorMessage } from "../lib/api";
import { CheckIcon, CopyIcon } from "./Icons";
import { Toggle } from "./Toggle";

interface ButtonProps {
  children: ReactNode;
  onClick: () => void;
  variant?: "primary" | "plain";
  disabled?: boolean;
  autoFocus?: boolean;
}

export function Button({ children, onClick, variant = "plain", disabled, autoFocus }: ButtonProps): ReactElement {
  const look =
    variant === "primary"
      ? "bg-accent text-[oklch(0.2_0.004_95)] hover:brightness-105 active:brightness-95"
      : "border border-line text-ink hover:bg-hover active:bg-press";
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      autoFocus={autoFocus}
      className={`h-9 min-w-24 rounded-lg px-4 text-[13px] font-medium outline-none transition duration-150 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent disabled:opacity-50 ${look}`}
    >
      {children}
    </button>
  );
}

export function Section({ title, note, children }: { title: string; note?: string; children: ReactNode }): ReactElement {
  return (
    <section className="flex flex-col">
      <h2 className="px-1 text-[13px] font-semibold text-ink">{title}</h2>
      {note && <p className="mt-0.5 px-1 text-[12.5px] leading-snug text-ink-3">{note}</p>}
      <div className={`${note ? "mt-2" : "mt-1.5"} flex flex-col rounded-lg border border-line`}>{children}</div>
    </section>
  );
}

interface ChoiceRowProps {
  title: string;
  detail?: string;
  mono?: boolean;
  checked: boolean;
  disabled?: boolean;
  first: boolean;
  onChange: (checked: boolean) => void;
}

export function ChoiceRow({ title, detail, mono, checked, disabled, first, onChange }: ChoiceRowProps): ReactElement {
  return (
    <label className={`flex items-center gap-3 px-3 py-2.5 ${first ? "" : "border-t border-line"}`}>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-[13.5px] font-medium text-ink">{title}</span>
        {detail && (
          <span
            className={`block truncate text-[11.5px] text-ink-3 ${mono ? "font-mono text-[11px]" : ""}`}
            title={detail}
          >
            {detail}
          </span>
        )}
      </span>
      <Toggle checked={checked} label={title} disabled={disabled} onChange={onChange} />
    </label>
  );
}

export function Note({ children, tone = "plain" }: { children: ReactNode; tone?: "plain" | "warn" | "danger" }): ReactElement {
  const look =
    tone === "danger"
      ? "border-danger-line bg-danger-soft text-danger"
      : tone === "warn"
        ? "border-line bg-well text-warn"
        : "border-line bg-well text-ink-2";
  return <p className={`rounded-lg border px-3 py-2.5 text-[12.5px] leading-snug ${look}`}>{children}</p>;
}

export function CopyLine({ text }: { text: string }): ReactElement {
  const [state, setState] = useState<"idle" | "copied" | "failed">("idle");
  const copy = async (): Promise<void> => {
    try {
      await api.copyText(text);
      setState("copied");
    } catch (failure) {
      console.error(errorMessage(failure));
      setState("failed");
    }
    window.setTimeout(() => setState("idle"), 1800);
  };
  return (
    <div className="flex items-center gap-2 rounded-lg border border-line bg-well py-1.5 pr-1.5 pl-3">
      <code className="select-text min-w-0 flex-1 font-mono text-[11px] leading-snug break-all text-ink-2">
        {text}
      </code>
      <button
        type="button"
        onClick={() => void copy()}
        aria-label="Copy command"
        className="grid h-7 w-7 shrink-0 place-items-center rounded-md text-ink-3 outline-none transition-colors hover:bg-hover hover:text-ink focus-visible:outline-2 focus-visible:outline-accent"
      >
        {state === "copied" ? <CheckIcon className="text-accent-ink" /> : <CopyIcon />}
      </button>
      {state === "failed" && <span className="pr-1 text-[11px] text-danger">Copy failed</span>}
    </div>
  );
}
