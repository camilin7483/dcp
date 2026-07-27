#!/bin/bash
# DCP Health Check — run from cron/systemd timer
# Checks that dcpd is running and responsive

set -euo pipefail

if ! command -v dcp &>/dev/null; then
    echo "dcp CLI not found"
    exit 1
fi

if ! dcp status &>/dev/null; then
    notify-send -u critical "DCP Daemon is down" "Restarting daemon..."
    systemctl --user restart dcpd 2>/dev/null || \
        (pkill dcpd 2>/dev/null; dcpd --foreground &>/dev/null &)
    echo "dcpd restarted at $(date)"
    exit 1
fi

# Log health
echo "$(date): dcpd OK" >> /tmp/dcp-health.log
