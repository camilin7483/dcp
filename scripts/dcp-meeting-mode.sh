#!/bin/bash
# Detect meeting apps and toggle Do Not Disturb
# Run as: dcp meeting-mode start|stop

start() {
    echo "Starting meeting mode watcher..."
    touch /tmp/dcp-meeting-mode.lock
    while [ -f /tmp/dcp-meeting-mode.lock ]; do
        ACTIVE=$(dcp query activeWindow --format=json 2>/dev/null | jq -r '.activeWindow.application' 2>/dev/null)
        case "$ACTIVE" in
            *zoom*|*Zoom*|*teams*|*Teams*|*meet*|*Meet*|*discord*|*Discord*|*slack*|*Slack*)
                if [ ! -f /tmp/dcp-dnd-active ]; then
                    notify-send "Meeting detected" "Silencing notifications for $ACTIVE"
                    touch /tmp/dcp-dnd-active
                    # Mute system notifications via D-Bus
                    busctl call org.freedesktop.Notifications /org/freedesktop/Notifications \
                        org.freedesktop.Notifications.CloseAllNotifications 2>/dev/null || true
                fi
                ;;
            *)
                if [ -f /tmp/dcp-dnd-active ]; then
                    notify-send "Meeting ended" "Restoring notifications"
                    rm -f /tmp/dcp-dnd-active
                fi
                ;;
        esac
        sleep 5
    done
}

stop() {
    echo "Stopping meeting mode..."
    rm -f /tmp/dcp-meeting-mode.lock /tmp/dcp-dnd-active
}

case "${1:-start}" in
    start) start ;;
    stop) stop ;;
    *) echo "Usage: $0 {start|stop}"; exit 1 ;;
esac
