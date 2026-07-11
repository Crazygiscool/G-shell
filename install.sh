#!/bin/sh
set -e

BIN_DIR="${1:-$HOME/.local/bin}"
BIN_NAME="g-shell"
PROFILE="release"

echo "Building G-Shell ($PROFILE)..."
cargo build --profile "$PROFILE"

mkdir -p "$BIN_DIR"
cp "target/$PROFILE/$BIN_NAME" "$BIN_DIR/"

if [ ! -f "$HOME/.gshellrc" ]; then
	echo "Copying default .gshellrc to $HOME/.gshellrc"
	cp .gshellrc "$HOME/.gshellrc"
else
	echo "~/.gshellrc already exists — skipping"
fi

echo "Installed to $BIN_DIR/$BIN_NAME"
echo ""
echo "Make sure $BIN_DIR is on your PATH, then run:"
echo "  g-shell"
echo ""
echo "To uninstall:"
echo "  rm $BIN_DIR/$BIN_NAME"
