# Delta — VS Code Extension

Launches the Delta TUI directly inside VS Code's integrated terminal.

## Workflow

```
workspace root
├── justfile / Makefile   ← build automation
└── vscode-extension/
    └── bin/
        └── delta.exe     ← populated by `just build`
```

### Build the binary + extension in one step

```powershell
# Preferred (requires `just`)
just package

# Fallback
make package
```

`just build` alone is enough to get `delta.exe` in place for testing.

### Test without packaging

1. Open the `vscode-extension/` folder in VS Code
2. Run `npm install` then press **F5** — an Extension Development Host opens
3. Hit **Ctrl+Alt+D** or search **"Launch Delta"** in the command palette

### Distribute

```powershell
just package          # produces delta-0.1.0.vsix
```

Share the `.vsix` — recipients install it via  
*Extensions panel → ··· → Install from VSIX…*

## Settings

| Setting | Default | Description |
|---|---|---|
| `delta.reuseTerminal` | `true` | Reuse the existing terminal tab |
| `delta.terminalName` | `"Delta"` | Label on the terminal tab |

## Keybinding

**Ctrl+Alt+D** (when terminal is not focused)
