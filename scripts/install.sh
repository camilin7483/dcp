#!/bin/bash
# Install DCP daemon for the current user.

set -euo pipefail

INSTALL_DIR="$HOME/.local/bin"
CONFIG_DIR="$HOME/.config/dcpd"
DATA_DIR="$HOME/.local/share/dcpd"
SYSTEMD_DIR="$HOME/.config/systemd/user"

echo "Installing DCP daemon..."

# Build release binary
echo "Building release binary..."
cargo build --release --workspace

# Install binaries
mkdir -p "$INSTALL_DIR"
cp target/release/dcpd "$INSTALL_DIR/"
cp target/release/dcp "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/dcpd" "$INSTALL_DIR/dcp"

# Create config directory and example config
mkdir -p "$CONFIG_DIR"
if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    cp scripts/config.example.toml "$CONFIG_DIR/config.toml"
    echo "Created config at $CONFIG_DIR/config.toml"
fi

# Create data directories
mkdir -p "$DATA_DIR/plugins"
mkdir -p "$DATA_DIR/audit"

# Install plugin binaries
mkdir -p "$DATA_DIR/plugins"
for plugin_dir in target/release/plugins/*/; do
    plugin_name=$(basename "$plugin_dir")
    plugin_bin=$(basename "$plugin_dir")
    install_dir="$DATA_DIR/plugins/$plugin_name"
    mkdir -p "$install_dir"
    cp "$plugin_dir/$plugin_name" "$install_dir/" 2>/dev/null || true
done

# Install systemd service
mkdir -p "$SYSTEMD_DIR"
cp scripts/dcpd.service "$SYSTEMD_DIR/"
systemctl --user daemon-reload

echo ""
echo "Installation complete!"
echo ""
echo "Start the daemon with:"
echo "  systemctl --user start dcpd"
echo ""
echo "Enable on boot with:"
echo "  systemctl --user enable dcpd"
echo ""
echo "Or run directly:"
echo "  dcpd"
echo ""
echo "Query context:"
echo "  dcp query activeWindow clipboard"
echo ""
echo "Full inspect:"
echo "  dcp inspect"
