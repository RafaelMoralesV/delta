# ── Delta workspace Makefile ──────────────────────────────────────────────────
# Fallback for environments without `just`.
#
# Usage:
#   make            → build Rust release + copy delta.exe into extension
#   make ext        → npm install + compile the VS Code extension TypeScript
#   make package    → full pipeline: build → ext → vsce package → VSIX
#   make clean      → cargo clean + remove copied binary

BIN_SRC := target/release/delta.exe
BIN_DST := vscode-extension/bin/delta.exe
EXT_DIR := vscode-extension

.PHONY: default build ext package clean

default: build

build:
	cargo build --release
	cp "$(BIN_SRC)" "$(BIN_DST)"
	@echo "delta.exe ready at $(BIN_DST)"

ext:
	cd $(EXT_DIR) && npm install
	cd $(EXT_DIR) && npm run compile

package: build ext
	cd $(EXT_DIR) && npm run package
	@echo ".vsix ready in $(EXT_DIR)/"

clean:
	cargo clean
	rm -f "$(BIN_DST)"
