#!/bin/bash
# Log active window to ~/focus.log every N seconds
# Usage: dcp-focus-tracker.sh [interval_seconds=60]

INTERVAL=${1:-60}
LOGFILE="$HOME/focus.log"

mkdir -p "$(dirname "$LOGFILE")"

echo "# Focus Tracker started $(date)" >> "$LOGFILE"

while true; do
    TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')
    WINDOW=$(dcp query activeWindow --format=json 2>/dev/null | jq -r '.activeWindow.title + " (" + .activeWindow.application + ")"' 2>/dev/null || echo "unknown")
    echo "$TIMESTAMP | $WINDOW" >> "$LOGFILE"
    sleep "$INTERVAL"
done
