import { useEffect, useState, type ReactElement, type ReactNode } from "react";
import { api, errorMessage, type BrowserView, type ListsStatus } from "../lib/api";
import { capitalise, formatCount, relativeTime } from "../lib/format";
import { GlobeIcon } from "./Icons";
import { Toggle } from "./Toggle";

interface SettingsProps {
  browsers: BrowserView[];
  lists: ListsStatus;
  blocklistEnabled: boolean;
  version: string;
  onBrowsersChange: (browsers: BrowserView[]) => void;
  onListsChange: (lists: ListsStatus) => void;
  onBlocklistEnabledChange: (enabled: boolean) => void;
}

function Section({ title, note, children }: { title: string; note: string; children: ReactNode }): ReactElement {
  return (
    <section className="flex flex-col">
      <h2 className="px-1 text-[13px] font-semibold text-ink">{title}</h2>
      <p className="mt-0.5 mb-2 px-1 text-[12.5px] leading-snug text-ink-3">{note}</p>
      <div className="flex flex-col rounded-lg border border-line">{children}</div>
    </section>
  );
}

export function Settings({
  browsers,
  lists,
  blocklistEnabled,
  version,
  onBrowsersChange,
  onListsChange,
  onBlocklistEnabledChange,
}: SettingsProps): ReactElement {
  const [saving, setSaving] = useState<string | null>(null);
  const [savingBlocklist, setSavingBlocklist] = useState(false);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const updating = checking || lists.updating;

  useEffect(() => {
    if (!blocklistEnabled || !lists.updating || checking) return;
    const timer = window.setInterval(() => {
      api.listsStatus().then(onListsChange, () => undefined);
    }, 2000);
    return () => window.clearInterval(timer);
  }, [blocklistEnabled, lists.updating, checking, onListsChange]);

  const toggle = async (browser: BrowserView, visible: boolean): Promise<void> => {
    setSaving(browser.id);
    setError(null);
    try {
      onBrowsersChange(await api.setBrowserHidden(browser.id, !visible));
    } catch (failure) {
      setError(errorMessage(failure));
    } finally {
      setSaving(null);
    }
  };

  const toggleBlocklist = async (enabled: boolean): Promise<void> => {
    setSavingBlocklist(true);
    setError(null);
    try {
      await api.setBlocklistEnabled(enabled);
      onBlocklistEnabledChange(enabled);
    } catch (failure) {
      setError(errorMessage(failure));
    } finally {
      setSavingBlocklist(false);
    }
  };

  const checkNow = async (): Promise<void> => {
    setChecking(true);
    setError(null);
    try {
      onListsChange(await api.updateLists());
    } catch (failure) {
      setError(errorMessage(failure));
      api.listsStatus().then(onListsChange, () => undefined);
    } finally {
      setChecking(false);
    }
  };

  return (
    <div className="animate-enter flex flex-col gap-6 px-4 pt-4 pb-4">
      <Section title="Browsers" note="Every browser installed on this PC. Hidden ones stay out of the picker.">
        {browsers.length === 0 && <p className="px-3 py-3 text-[13px] text-ink-2">No browsers found.</p>}
        {browsers.map((browser, index) => (
          <label
            key={browser.id}
            className={`flex items-center gap-3 px-3 py-2.5 ${index > 0 ? "border-t border-line" : ""}`}
          >
            <span className="grid h-5 w-5 shrink-0 place-items-center text-ink-2">
              {browser.icon ? <img src={browser.icon} alt="" className="h-5 w-5" draggable={false} /> : <GlobeIcon />}
            </span>
            <span className="min-w-0 flex-1">
              <span className="block truncate text-[13.5px] font-medium text-ink">{browser.name}</span>
              <span className="block truncate font-mono text-[11px] text-ink-3" title={browser.path}>
                {browser.path}
              </span>
            </span>
            <Toggle
              checked={!browser.hidden}
              label={`Show ${browser.name}`}
              disabled={saving === browser.id}
              onChange={(visible) => void toggle(browser, visible)}
            />
          </label>
        ))}
      </Section>

      <Section
        title="Blocklists"
        note="From Pi-hole Optimized Blocklists. A match never blocks a link, it adds a confirmation step."
      >
        <label className="flex items-center gap-3 px-3 py-2.5">
          <span className="min-w-0 flex-1">
            <span className="block text-[13.5px] font-medium text-ink">Check links against the blocklists</span>
            <span className="block text-[12px] text-ink-3">Looks up each link's domain in lists stored on this PC.</span>
          </span>
          <Toggle
            checked={blocklistEnabled}
            label="Check links against the blocklists"
            disabled={savingBlocklist}
            onChange={(enabled) => void toggleBlocklist(enabled)}
          />
        </label>
        {lists.lists.map((list) => (
          <div key={list.name} className="flex items-baseline justify-between gap-3 border-t border-line px-3 py-2.5">
            <span className="text-[13.5px] font-medium text-ink">{capitalise(list.name)}</span>
            <span className="text-[12.5px] text-ink-3 tabular-nums">
              {list.count ? `${formatCount(list.count)} domains` : "Not downloaded yet"}
            </span>
          </div>
        ))}
        <div className="flex items-center justify-between gap-3 border-t border-line px-3 py-2">
          <span className="text-[12.5px] text-ink-3">
            {!blocklistEnabled
              ? "Turned off"
              : updating
                ? "Downloading lists…"
                : `Checked ${relativeTime(lists.checkedAt)}`}
          </span>
          <button
            type="button"
            disabled={updating || !blocklistEnabled}
            onClick={() => void checkNow()}
            className="h-7 rounded-md border border-line px-2.5 text-[12.5px] font-medium text-ink-2 outline-none transition-colors duration-150 hover:bg-hover hover:text-ink focus-visible:outline-2 focus-visible:outline-accent active:bg-press disabled:opacity-50"
          >
            {updating ? "Checking…" : "Check now"}
          </button>
        </div>
      </Section>

      {(error ?? lists.lastError) && (
        <p role="alert" className="px-1 text-[12.5px] text-danger">
          {error ?? lists.lastError}
        </p>
      )}
      <p className="px-1 text-[11.5px] text-ink-3">linkgate {version}</p>
    </div>
  );
}
