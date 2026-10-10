import type { ReactElement } from "react";
import type { BrowserEnv, Distro, InitialState, InstallChoices, Readiness } from "../lib/api";
import { Button, ChoiceRow, CopyLine, Note, Section } from "../components/ui";

export type Latest = { state: "checking" } | { state: "ready"; version: string } | { state: "failed"; message: string };

interface OptionsProps {
  state: InitialState;
  latest: Latest;
  choices: InstallChoices;
  readiness: Record<string, Readiness> | null;
  onChoices: (choices: InstallChoices) => void;
  onRetry: () => void;
  onInstall: () => void;
  onCancel: () => void;
}

const ENV_OPTIONS: { value: BrowserEnv; label: string }[] = [
  { value: "off", label: "Off" },
  { value: "zshenv", label: "~/.zshenv" },
  { value: "profile", label: "~/.profile" },
];

function toggled(list: string[], item: string, on: boolean): string[] {
  const rest = list.filter((entry) => entry !== item);
  return on ? [...rest, item] : rest;
}

function LatestLine({ state, latest, onRetry }: { state: InitialState; latest: Latest; onRetry: () => void }): ReactElement {
  if (latest.state === "checking") return <p className="text-[12.5px] text-ink-3">Checking the latest release…</p>;
  if (latest.state === "failed") {
    return (
      <div className="flex flex-col gap-2">
        <Note tone="danger">{latest.message}</Note>
        <div>
          <Button onClick={onRetry}>Try again</Button>
        </div>
      </div>
    );
  }
  const installed = state.installedVersion;
  const text =
    installed === null
      ? `Latest release: ${latest.version}`
      : installed === latest.version
        ? `Installed ${installed}, which is the latest. Running this again reinstalls it.`
        : `Installed ${installed}. Latest release: ${latest.version}`;
  return <p className="text-[12.5px] text-ink-3">{text}</p>;
}

function DistroChecks({
  distro,
  readiness,
  choices,
  onChange,
}: {
  distro: Distro;
  readiness: Record<string, Readiness> | null;
  choices: InstallChoices;
  onChange: (installPackages: string[]) => void;
}): ReactElement | null {
  if (readiness === null) {
    return (
      <p className="border-t border-line px-3 py-2 text-[12px] text-ink-3">
        Checking what {distro.name} has installed…
      </p>
    );
  }
  const found = readiness[distro.name];
  if (!found) return null;
  const chosen = choices.wslDistros.includes(distro.name);
  return (
    <>
      {found.probeError && (
        <p className="border-t border-line px-3 py-2 text-[12.5px] leading-snug text-warn">
          Couldn't check {distro.name}: {found.probeError}
        </p>
      )}
      {found.issue && (
        <p className="border-t border-line px-3 py-2 text-[12.5px] leading-snug text-warn">{found.issue}</p>
      )}
      {found.manual && (
        <p className="border-t border-line px-3 py-2 text-[12.5px] leading-snug text-ink-3">{found.manual}</p>
      )}
      {found.offer && chosen && (
        <ChoiceRow
          title={found.offer.label}
          detail={found.offer.command}
          mono
          wrapDetail
          first={false}
          checked={choices.installPackages.includes(distro.name)}
          onChange={(on) => onChange(toggled(choices.installPackages, distro.name, on))}
        />
      )}
    </>
  );
}

export function Options({
  state,
  latest,
  choices,
  readiness,
  onChoices,
  onRetry,
  onInstall,
  onCancel,
}: OptionsProps): ReactElement {
  const update = state.mode === "update";
  const set = (patch: Partial<InstallChoices>): void => onChoices({ ...choices, ...patch });
  const availableVsCode = state.vscode.filter((target) => target.available);
  const wslChosen = choices.wslDistros.length > 0;
  const checking = wslChosen && readiness === null;

  return (
    <>
      <div className="animate-enter flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-4 pt-5 pb-4">
        <div className="flex flex-col gap-1.5 px-1">
          <h1 className="text-[20px] leading-tight font-semibold tracking-tight text-ink">
            {update ? "Update linkgate" : "Install linkgate"}
          </h1>
          <LatestLine state={state} latest={latest} onRetry={onRetry} />
        </div>

        {state.sandboxed && (
          <Note tone="warn">Test mode. Everything goes to a scratch folder and your real setup is not touched.</Note>
        )}
        {state.localSource && <Note>Using a local release folder instead of GitHub.</Note>}

        <Section title="Shortcuts" note="So you can open settings without finding the program.">
          <ChoiceRow title="Start menu" detail="Always added" checked disabled first onChange={() => undefined} />
          <ChoiceRow
            title="Desktop"
            first={false}
            checked={choices.desktopShortcut}
            onChange={(on) => set({ desktopShortcut: on })}
          />
        </Section>

        <Section
          title="WSL"
          note="Makes xdg-open in a distribution show the picker for links, PDFs and images."
        >
          {state.wsl.distros.length === 0 && (
            <p className="px-3 py-3 text-[13px] text-ink-2">{state.wsl.note ?? "No WSL distributions found."}</p>
          )}
          {state.wsl.distros.map((distro, index) => (
            <div key={distro.name} className="flex flex-col">
              <ChoiceRow
                title={distro.name}
                detail={distro.isDefault ? "Default distribution" : undefined}
                first={index === 0}
                checked={choices.wslDistros.includes(distro.name)}
                onChange={(on) => set({ wslDistros: toggled(choices.wslDistros, distro.name, on) })}
              />
              <DistroChecks
                distro={distro}
                readiness={readiness}
                choices={choices}
                onChange={(installPackages) => set({ installPackages })}
              />
            </div>
          ))}
        </Section>

        {wslChosen && (
          <Section
            title="Programs that read $BROWSER"
            note="Some tools ignore xdg-open and use this variable. The line is added once and takes effect in new shells."
          >
            <div className="flex gap-1.5 p-2" role="radiogroup" aria-label="Where to set BROWSER">
              {ENV_OPTIONS.map((option) => (
                <button
                  key={option.value}
                  type="button"
                  role="radio"
                  aria-checked={choices.browserEnv === option.value}
                  onClick={() => set({ browserEnv: option.value })}
                  className={`h-8 flex-1 rounded-md font-mono text-[12px] outline-none transition-colors focus-visible:outline-2 focus-visible:outline-accent ${
                    choices.browserEnv === option.value ? "bg-press text-ink" : "text-ink-3 hover:bg-hover"
                  }`}
                >
                  {option.label}
                </button>
              ))}
            </div>
            <div className="flex flex-col gap-2 border-t border-line p-3">
              <p className="text-[12.5px] leading-snug text-ink-3">
                Programs that use <span className="font-mono">x-www-browser</span> need one command with sudo. The
                installer never runs it. Copy it into your distribution if you want it.
              </p>
              <CopyLine text={state.alternativesCommand} />
            </div>
          </Section>
        )}

        <Section
          title="VS Code"
          note="Points VS Code's links at linkgate and keeps localhost links out of its built-in browser. Your settings file is backed up first."
        >
          {availableVsCode.length === 0 && (
            <p className="px-3 py-3 text-[13px] text-ink-2">No VS Code settings folder was found.</p>
          )}
          {availableVsCode.map((target, index) => (
            <ChoiceRow
              key={target.id}
              title={target.label}
              detail={target.path}
              mono
              first={index === 0}
              checked={choices.vscode.includes(target.id)}
              onChange={(on) => set({ vscode: toggled(choices.vscode, target.id, on) })}
            />
          ))}
        </Section>

        <p className="px-1 text-[12px] leading-snug text-ink-3">
          Installs to <span className="font-mono break-all">{state.installDir}</span>. Anything left unticked is not changed.
        </p>
      </div>
      <footer className="flex shrink-0 justify-end gap-2 border-t border-line bg-chrome px-4 py-3">
        <Button onClick={onCancel}>Cancel</Button>
        <Button variant="primary" onClick={onInstall} disabled={latest.state === "checking" || checking} autoFocus>
          {update ? "Update" : "Install"}
        </Button>
      </footer>
    </>
  );
}
