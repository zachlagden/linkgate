import { useCallback, useEffect, useMemo, useState, type ReactElement } from "react";
import {
  api,
  errorMessage,
  type InitialState,
  type InstallChoices,
  type Readiness,
  type StepPlan,
  type StepResult,
  type StepStatus,
  type Summary,
  type UninstallChoices,
} from "./lib/api";
import { TitleBar } from "./components/TitleBar";
import { Done } from "./screens/Done";
import { Options, type Latest } from "./screens/Options";
import { Progress, type DownloadProgress, type StepRowData } from "./screens/Progress";
import { UninstallOptions } from "./screens/UninstallOptions";

type Screen = "options" | "running" | "done";

interface StepState {
  status: StepStatus;
  message: string | null;
}

function defaultChoices(state: InitialState): InstallChoices {
  const previous = state.previous;
  const distroNames = state.wsl.distros.map((distro) => distro.name);
  if (previous) {
    return {
      desktopShortcut: previous.desktopShortcut,
      wslDistros: previous.wsl.filter((name) => distroNames.includes(name)),
      installPackages: [],
      browserEnv: previous.browserEnv,
      vscode: previous.vscode.filter((id) => state.vscode.some((target) => target.id === id && target.available)),
    };
  }
  const preferred = state.wsl.distros.find((distro) => distro.isDefault);
  return {
    desktopShortcut: false,
    wslDistros: preferred ? [preferred.name] : [],
    installPackages: [],
    browserEnv: "off",
    vscode: [],
  };
}

function rowsFrom(plan: StepPlan[], states: Record<string, StepState>, summary: Summary | null): StepRowData[] {
  if (summary) {
    return summary.steps.map((step: StepResult) => ({
      id: step.id,
      label: step.label,
      status: step.status,
      message: step.message,
    }));
  }
  return plan.map((step) => ({
    id: step.id,
    label: step.label,
    status: states[step.id]?.status ?? "pending",
    message: states[step.id]?.message ?? null,
  }));
}

export function App(): ReactElement {
  const [state, setState] = useState<InitialState | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [latest, setLatest] = useState<Latest>({ state: "checking" });
  const [choices, setChoices] = useState<InstallChoices | null>(null);
  const [readiness, setReadiness] = useState<Record<string, Readiness> | null>(null);
  const [uninstallChoices, setUninstallChoices] = useState<UninstallChoices>({
    restoreVscode: false,
    removeData: false,
    removePackages: [],
  });
  const [screen, setScreen] = useState<Screen>("options");
  const [plan, setPlan] = useState<StepPlan[]>([]);
  const [stepStates, setStepStates] = useState<Record<string, StepState>>({});
  const [download, setDownload] = useState<DownloadProgress | null>(null);
  const [summary, setSummary] = useState<Summary | null>(null);
  const [fatal, setFatal] = useState<string | null>(null);

  const checkLatest = useCallback(() => {
    setLatest({ state: "checking" });
    api.checkLatest().then(
      (release) => setLatest({ state: "ready", version: release.version }),
      (failure) => setLatest({ state: "failed", message: errorMessage(failure) }),
    );
  }, []);

  useEffect(() => {
    api.initialState().then(
      (initial) => {
        setState(initial);
        setChoices(defaultChoices(initial));
        if (initial.mode !== "uninstall") checkLatest();
      },
      (failure) => setLoadError(errorMessage(failure)),
    );
  }, [checkLatest]);

  useEffect(() => {
    if (state === null) return;
    if (state.mode === "uninstall" || state.wsl.distros.length === 0) {
      setReadiness({});
      return;
    }
    let current = true;
    api.probeDistros(state.wsl.distros.map((distro) => distro.name)).then(
      (results) => {
        if (!current) return;
        setReadiness(Object.fromEntries(results.map((result) => [result.name, result.readiness])));
        const offered = results.filter((result) => result.readiness.offer !== null).map((result) => result.name);
        setChoices((existing) => (existing ? { ...existing, installPackages: offered } : existing));
      },
      (failure) => {
        console.error(errorMessage(failure));
        if (current) setReadiness({});
      },
    );
    return () => {
      current = false;
    };
  }, [state]);

  useEffect(() => {
    if (state !== null || loadError !== null) void api.present();
  }, [state, loadError]);

  const running = screen === "running";

  useEffect(() => {
    if (running) return;
    const onKey = (event: KeyboardEvent): void => {
      if (event.key === "Escape") void api.dismiss();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [running]);

  const run = useCallback(async (start: () => Promise<Summary>): Promise<void> => {
    setScreen("running");
    setPlan([]);
    setStepStates({});
    setDownload(null);
    setSummary(null);
    setFatal(null);
    const unlisten = await api.onProgress((event) => {
      if (event.kind === "plan") setPlan(event.steps);
      else if (event.kind === "download") setDownload({ done: event.done, total: event.total });
      else setStepStates((current) => ({ ...current, [event.id]: { status: event.status, message: event.message } }));
    });
    try {
      setSummary(await start());
    } catch (failure) {
      setFatal(errorMessage(failure));
    } finally {
      unlisten();
      setScreen("done");
    }
  }, []);

  const rows = useMemo(() => rowsFrom(plan, stepStates, summary), [plan, stepStates, summary]);
  const appInstalled = rows.some((row) => row.id === "app" && row.status === "done");

  const title = state?.mode === "uninstall" ? "linkgate uninstall" : "linkgate setup";
  const mode = state?.mode ?? "install";

  return (
    <div className="flex h-screen flex-col">
      <TitleBar title={title} canClose={!running} />
      {loadError && <p className="px-5 py-6 text-[13px] text-danger select-text">{loadError}</p>}
      {state && choices && screen === "options" && mode !== "uninstall" && (
        <Options
          state={state}
          latest={latest}
          choices={choices}
          readiness={readiness}
          onChoices={setChoices}
          onRetry={checkLatest}
          onInstall={() => void run(() => api.runInstall(choices))}
          onCancel={() => void api.dismiss()}
        />
      )}
      {state && screen === "options" && mode === "uninstall" && (
        <UninstallOptions
          state={state}
          choices={uninstallChoices}
          onChoices={setUninstallChoices}
          onUninstall={() => void run(() => api.runUninstall(uninstallChoices))}
          onCancel={() => void api.dismiss()}
        />
      )}
      {running && (
        <Progress
          title={mode === "uninstall" ? "Uninstalling linkgate…" : mode === "update" ? "Updating linkgate…" : "Installing linkgate…"}
          steps={rows}
          download={download}
        />
      )}
      {screen === "done" && (
        <Done
          mode={mode}
          version={summary?.version ?? null}
          steps={rows}
          fatal={fatal}
          canOpen={mode !== "uninstall" && appInstalled}
          onOpen={() => void api.openLinkgate()}
          onClose={() => void api.dismiss()}
        />
      )}
    </div>
  );
}
