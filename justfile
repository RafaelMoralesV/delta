# ── Delta workspace justfile ─────────────────────────────────────────────────
# Install just:  cargo install just   or   winget install Casey.Just
#
# Usage:
#   just            → build Rust release + copy delta.exe into extension
#   just build      → same
#   just ext        → npm install + compile the VS Code extension TypeScript
#   just package    → full pipeline: build → ext → vsce package → VSIX
#   just clean      → cargo clean + remove copied binary

# Paths
bin_src := "target/release/delta.exe"
bin_dst := "vscode-extension/bin/delta.exe"
ext_dir := "vscode-extension"

# ── Default ──────────────────────────────────────────────────────────────────

default: build

# ── Rust build ───────────────────────────────────────────────────────────────

# Build release binary and copy it as delta.exe into the extension's bin/
build:
    cargo build --release
    cp "{{bin_src}}" "{{bin_dst}}"
    @echo "delta.exe ready at {{bin_dst}}"

# ── VS Code extension ────────────────────────────────────────────────────────

# Install npm deps and compile TypeScript
ext:
    cd {{ext_dir}} && npm install
    cd {{ext_dir}} && npm run compile

# ── Full package ─────────────────────────────────────────────────────────────

# Rust release → copy binary → compile extension → produce .vsix
package: build ext
    cd {{ext_dir}} && npm run package
    @echo ".vsix ready in {{ext_dir}}/"

# ── Utility ──────────────────────────────────────────────────────────────────

clean:
    cargo clean
    rm -f "{{bin_dst}}"
