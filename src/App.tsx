import { useCallback, useEffect, useMemo, useRef, useState, type ReactElement } from "react";
import { api, errorMessage, type BrowserView, type InitialState, type ListsStatus } from "./lib/api";
import { useWindowSizing } from "./lib/useWindowSizing";
import { Countdown } from "./components/Countdown";
import { Picker } from "./components/Picker";
import { Settings } from "./components/Settings";
import { TitleBar } from "./components/TitleBar";

const COUNTDOWN_MS = 10_000;

type View = "picker" | "settings";

export function App(): ReactElement {
  const rootRef = useRef<HTMLDivElement>(null);
  const [state, setState] = useState<InitialState | null>(null);
  const [browsers, setBrowsers] = useState<BrowserView[]>([]);
  const [lists, setLists] = useState<ListsStatus | null>(null);
  const [view, setView] = useState<View>("picker");
  const [countdownKey, setCountdownKey] = useState(0);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [pointerInside, setPointerInside] = useState(false);

  useEffect(() => {
    api.initialState().then(
      (initial) => {
        setState(initial);
        setBrowsers(initial.browsers);
        setLists(initial.lists);
        if (!initial.raw) setView("settings");
      },
      (failure) => setLoadError(errorMessage(failure)),
    );
  }, []);

  useWindowSizing(rootRef, state !== null || loadError !== null);

  useEffect(() => {
    const enter = (): void => setPointerInside(true);
    const leave = (): void => setPointerInside(false);
    document.addEventListener("pointermove", enter);
    document.documentElement.addEventListener("pointerleave", leave);
    window.addEventListener("blur", leave);
    return () => {
      document.removeEventListener("pointermove", enter);
      document.documentElement.removeEventListener("pointerleave", leave);
      window.removeEventListener("blur", leave);
    };
  }, []);

  const hasLink = Boolean(state?.raw);
  const visibleBrowsers = useMemo(() => browsers.filter((browser) => !browser.hidden), [browsers]);
  const restartCountdown = useCallback(() => setCountdownKey((key) => key + 1), []);
  const dismiss = useCallback(() => void api.dismiss(), []);

  const backToPicker = useCallback(() => {
    setView("picker");
    restartCountdown();
  }, [restartCountdown]);

  useEffect(() => {
    if (view !== "settings") return;
    const onKey = (event: KeyboardEvent): void => {
      if (event.key !== "Escape") return;
      if (hasLink) backToPicker();
      else dismiss();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [view, hasLink, backToPicker, dismiss]);

  const inPicker = view === "picker" && hasLink;

  return (
    <div ref={rootRef} className="flex max-h-[680px] flex-col">
      <TitleBar
        title={inPicker ? "Open link" : "linkgate settings"}
        onClose={dismiss}
        onBack={!inPicker && hasLink ? backToPicker : undefined}
        onSettings={inPicker ? () => setView("settings") : undefined}
      />
      <Countdown
        runKey={countdownKey}
        durationMs={COUNTDOWN_MS}
        running={inPicker}
        paused={pointerInside}
        onElapsed={dismiss}
      />
      <main className="min-h-0 overflow-y-auto">
        {loadError && <p className="px-5 py-6 text-[13px] text-danger">{loadError}</p>}
        {state && lists && inPicker && (
          <Picker
            key="picker"
            state={state}
            browsers={visibleBrowsers}
            onSettings={() => setView("settings")}
            onRestartCountdown={restartCountdown}
          />
        )}
        {state && lists && !inPicker && (
          <Settings
            browsers={browsers}
            lists={lists}
            version={state.version}
            onBrowsersChange={setBrowsers}
            onListsChange={setLists}
          />
        )}
      </main>
    </div>
  );
}
