import type { ReactElement } from "react";
import { BackIcon, CloseIcon, GearIcon, Mark } from "./Icons";

interface TitleBarProps {
  title: string;
  onClose: () => void;
  onBack?: () => void;
  onSettings?: () => void;
}

const iconButton =
  "grid h-7 w-8 place-items-center rounded-md text-ink-3 outline-none transition-colors duration-150 hover:bg-hover hover:text-ink active:bg-press focus-visible:outline-2 focus-visible:outline-accent";

export function TitleBar({ title, onClose, onBack, onSettings }: TitleBarProps): ReactElement {
  return (
    <header
      data-tauri-drag-region
      className="flex h-10 shrink-0 items-center gap-2 bg-chrome pr-1.5 pl-3"
    >
      {onBack ? (
        <button type="button" className={`${iconButton} -ml-1.5`} onClick={onBack} aria-label="Back">
          <BackIcon />
        </button>
      ) : (
        <Mark className="shrink-0" />
      )}
      <span data-tauri-drag-region className="flex-1 truncate text-[12.5px] font-medium text-ink-2">
        {title}
      </span>
      {onSettings && (
        <button type="button" className={iconButton} onClick={onSettings} aria-label="Settings">
          <GearIcon />
        </button>
      )}
      <button
        type="button"
        className={`${iconButton} hover:bg-danger hover:text-bg active:bg-danger`}
        onClick={onClose}
        aria-label="Close"
      >
        <CloseIcon />
      </button>
    </header>
  );
}
