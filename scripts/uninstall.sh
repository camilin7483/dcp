#!/bin/bash
# Uninstall DCP daemon.

set -euo pipefail

echo "Stopping DCP daemon..."
systemctl --user stop dcpd 2>/dev/null || true
systemctl --user disable dcpd 2>/dev/null || true

echo "Removing binaries..."
rm -f "$HOME/.local/bin/dcpd"
rm -f "$HOME/.local/bin/dcp"

echo "Removing systemd service..."
rm -f "$HOME/.config/systemd/user/dcpd.service"
systemctl --user daemon-reload

echo "Removing socket..."
rm -f "${XDG_RUNTIME_DIR:-/tmp}/dcpd.sock"

echo ""
echo "Uninstalled. Data and config preserved at:"
echo "  Config: $HOME/.config/dcpd/"
echo "  Data:   $HOME/.local/share/dcpd/"
echo ""
echo "To fully remove:"
echo "  rm -rf ~/.config/dcpd ~/.local/share/dcpd"
