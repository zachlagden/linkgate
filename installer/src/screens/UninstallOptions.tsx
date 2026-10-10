import type { ReactElement } from "react";
import type { InitialState, UninstallChoices } from "../lib/api";
import { Button, ChoiceRow, Note, Section } from "../components/ui";

interface UninstallOptionsProps {
  state: InitialState;
  choices: UninstallChoices;
  onChoices: (choices: UninstallChoices) => void;
  onUninstall: () => void;
  onCancel: () => void;
}

export function UninstallOptions({ state, choices, onChoices, onUninstall, onCancel }: UninstallOptionsProps): ReactElement {
  const previous = state.previous;
  const installedPackages = previous?.installedPackages ?? [];
  const vscodeLabels = (previous?.vscode ?? []).map(
    (id) => state.vscode.find((target) => target.id === id)?.label ?? id,
  );
  return (
    <>
      <div className="animate-enter flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-4 pt-5 pb-4">
        <div className="flex flex-col gap-1.5 px-1">
          <h1 className="text-[20px] leading-tight font-semibold tracking-tight text-ink">Uninstall linkgate</h1>
          <p className="text-[12.5px] text-ink-3">
            {state.installedVersion ? `Installed version ${state.installedVersion}.` : "No install record was found."}
          </p>
        </div>

        {state.sandboxed && (
          <Note tone="warn">Test mode. Everything goes to a scratch folder and your real setup is not touched.</Note>
        )}

        <Section title="This removes" note="Everything the installer put on this PC.">
          <p className="px-3 py-2.5 text-[13px] break-words text-ink">The program and the installer copy in {state.installDir}</p>
          <p className="border-t border-line px-3 py-2.5 text-[13px] text-ink">
            The Start menu and desktop shortcuts, and the entry in Windows Settings
          </p>
          {(previous?.wsl ?? []).map((distro) => (
            <p key={distro} className="border-t border-line px-3 py-2.5 text-[13px] text-ink">
              The WSL setup in {distro}
            </p>
          ))}
        </Section>

        <Section title="Optional">
          <ChoiceRow
            title="Delete blocklists and settings"
            detail="Your hidden browsers and the downloaded lists"
            first
            checked={choices.removeData}
            onChange={(on) => onChoices({ ...choices, removeData: on })}
          />
          {installedPackages.map((item) => (
            <ChoiceRow
              key={item.distro}
              title={`Remove ${item.packages.join(" and ")} from ${item.distro}`}
              detail="linkgate installed it. Other programs there may use it too."
              first={false}
              checked={choices.removePackages.includes(item.distro)}
              onChange={(on) =>
                onChoices({
                  ...choices,
                  removePackages: on
                    ? [...choices.removePackages.filter((name) => name !== item.distro), item.distro]
                    : choices.removePackages.filter((name) => name !== item.distro),
                })
              }
            />
          ))}
          {vscodeLabels.length > 0 && (
            <ChoiceRow
              title="Restore VS Code settings"
              detail={`Undoes the two linkgate settings in ${vscodeLabels.join(" and ")}`}
              first={false}
              checked={choices.restoreVscode}
              onChange={(on) => onChoices({ ...choices, restoreVscode: on })}
            />
          )}
        </Section>

        {vscodeLabels.length > 0 && !choices.restoreVscode && (
          <Note tone="warn">
            VS Code keeps pointing at a program that no longer exists unless you restore its settings.
          </Note>
        )}
      </div>
      <footer className="flex shrink-0 justify-end gap-2 border-t border-line bg-chrome px-4 py-3">
        <Button onClick={onCancel} autoFocus>
          Cancel
        </Button>
        <Button variant="primary" onClick={onUninstall}>
          Uninstall
        </Button>
      </footer>
    </>
  );
}
