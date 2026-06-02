import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

let activeTerminal: vscode.Terminal | undefined;

export function activate(context: vscode.ExtensionContext) {
  const launch = vscode.commands.registerCommand('delta.launch', () => {
    launchDelta(context);
  });

  context.subscriptions.push(launch);

  context.subscriptions.push(
    vscode.window.onDidCloseTerminal((t) => {
      if (t === activeTerminal) {
        activeTerminal = undefined;
      }
    })
  );
}

function launchDelta(context: vscode.ExtensionContext) {
  const config = vscode.workspace.getConfiguration('delta');
  const reuse = config.get<boolean>('reuseTerminal', true);
  const name  = config.get<string>('terminalName', 'Delta');

  const exePath = context.asAbsolutePath(path.join('bin', 'delta.exe'));

  if (!fs.existsSync(exePath)) {
    vscode.window.showErrorMessage(
      `Delta binary not found at: ${exePath}\n` +
      `Run \`just build\` (or \`make build\`) in the workspace root first.`
    );
    return;
  }

  // Reuse existing terminal if still alive
  if (reuse && activeTerminal) {
    if (vscode.window.terminals.includes(activeTerminal)) {
      activeTerminal.show(true);
      activeTerminal.sendText(`"${exePath}"`, true);
      return;
    }
    activeTerminal = undefined;
  }

  const terminal = vscode.window.createTerminal({
    name,
    shellPath: 'cmd.exe',
    cwd: vscode.workspace.workspaceFolders?.[0]?.uri.fsPath,
  });

  activeTerminal = terminal;
  terminal.show(true);

  // Small delay so cmd.exe initialises before we send the command
  setTimeout(() => terminal.sendText(`"${exePath}"`, true), 300);
}

export function deactivate() {
  activeTerminal = undefined;
}
