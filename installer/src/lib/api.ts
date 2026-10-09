import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Mode = "install" | "update" | "uninstall";
export type BrowserEnv = "off" | "zshenv" | "profile";
export type StepStatus = "running" | "done" | "failed";

export interface Distro {
  name: string;
  isDefault: boolean;
}

export interface WslStatus {
  available: boolean;
  distros: Distro[];
  note: string | null;
}

export interface VsCodeView {
  id: string;
  label: string;
  path: string;
  available: boolean;
}

export interface Previous {
  desktopShortcut: boolean;
  wsl: string[];
  vscode: string[];
  browserEnv: BrowserEnv;
}

export interface InitialState {
  mode: Mode;
  setupVersion: string;
  installedVersion: string | null;
  installDir: string;
  sandboxed: boolean;
  localSource: boolean;
  wsl: WslStatus;
  vscode: VsCodeView[];
  previous: Previous | null;
  browserLine: string;
  alternativesCommand: string;
}

export interface InstallChoices {
  desktopShortcut: boolean;
  wslDistros: string[];
  browserEnv: BrowserEnv;
  vscode: string[];
}

export interface UninstallChoices {
  restoreVscode: boolean;
  removeData: boolean;
}

export interface StepPlan {
  id: string;
  label: string;
}

export interface StepResult {
  id: string;
  label: string;
  status: StepStatus;
  message: string | null;
}

export interface Summary {
  version: string;
  steps: StepResult[];
  failed: number;
}

export type ProgressEvent =
  | { kind: "plan"; steps: StepPlan[] }
  | { kind: "step"; id: string; status: StepStatus; message: string | null }
  | { kind: "download"; done: number; total: number | null };

export const api = {
  initialState: (): Promise<InitialState> => invoke("initial_state"),
  checkLatest: (): Promise<{ version: string }> => invoke("check_latest"),
  runInstall: (choices: InstallChoices): Promise<Summary> => invoke("run_install", { choices }),
  runUninstall: (choices: UninstallChoices): Promise<Summary> => invoke("run_uninstall", { choices }),
  copyText: (text: string): Promise<void> => invoke("copy_text", { text }),
  openLinkgate: (): Promise<void> => invoke("open_linkgate"),
  present: (): Promise<void> => invoke("present"),
  dismiss: (): Promise<void> => invoke("dismiss"),
  onProgress: (handler: (event: ProgressEvent) => void): Promise<UnlistenFn> =>
    listen<ProgressEvent>("setup://progress", (event) => handler(event.payload)),
};

export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "Something went wrong.";
}
