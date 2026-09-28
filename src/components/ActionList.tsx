import type { ReactElement, ReactNode } from "react";
import type { BrowserView } from "../lib/api";
import { CopyIcon, GlobeIcon } from "./Icons";
import { Kbd } from "./Kbd";

interface ActionListProps {
  browsers: BrowserView[];
  canOpen: boolean;
  busy: boolean;
  onOpen: (browser: BrowserView) => void;
  onCopy: () => void;
  onSettings: () => void;
}

interface RowProps {
  icon: ReactNode;
  label: string;
  hint: string;
  badge?: string;
  disabled: boolean;
  onClick: () => void;
}

function Row({ icon, label, hint, badge, disabled, onClick }: RowProps): ReactElement {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className="group flex h-11 w-full items-center gap-3 rounded-lg px-3 text-left outline-none transition-[background-color,transform] duration-150 ease-out-quart hover:bg-hover focus-visible:bg-hover focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent active:scale-[0.985] active:bg-press disabled:pointer-events-none disabled:opacity-45"
    >
      <span className="grid h-5 w-5 shrink-0 place-items-center text-ink-2">{icon}</span>
      <span className="flex-1 truncate text-[14px] font-medium text-ink">{label}</span>
      {badge && (
        <span className="rounded-[5px] bg-well px-1.5 py-0.5 text-[11px] text-ink-3">{badge}</span>
      )}
      <Kbd>{hint}</Kbd>
    </button>
  );
}

export function ActionList({ browsers, canOpen, busy, onOpen, onCopy, onSettings }: ActionListProps): ReactElement {
  return (
    <section className="flex flex-col">
      <h2 className="mb-1 px-3 text-[12px] font-medium text-ink-3">Open with</h2>
      {browsers.length === 0 ? (
        <p className="px-3 py-2 text-[13px] text-ink-2">
          Every browser is hidden.{" "}
          <button
            type="button"
            onClick={onSettings}
            className="font-medium text-accent-ink outline-none hover:underline focus-visible:underline"
          >
            Choose which to show
          </button>
        </p>
      ) : (
        browsers.map((browser, index) => (
          <Row
            key={browser.id}
            icon={
              browser.icon ? (
                <img src={browser.icon} alt="" className="h-5 w-5" draggable={false} />
              ) : (
                <GlobeIcon />
              )
            }
            label={browser.name}
            badge={browser.isDefault ? "Default" : undefined}
            hint={index < 9 ? String(index + 1) : ""}
            disabled={busy || !canOpen}
            onClick={() => onOpen(browser)}
          />
        ))
      )}
      <div className="mx-3 my-1.5 h-px bg-line" />
      <Row icon={<CopyIcon />} label="Copy link" hint="C" disabled={busy} onClick={onCopy} />
    </section>
  );
}
