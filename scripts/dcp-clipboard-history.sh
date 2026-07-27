#!/bin/bash
# Watch clipboard and save history to SQLite
# Dependencies: sqlite3

DB="$HOME/.local/share/dcpd/clipboard-history.db"
mkdir -p "$(dirname "$DB")"

# Init DB
sqlite3 "$DB" "CREATE TABLE IF NOT EXISTS history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT NOT NULL,
    app TEXT,
    timestamp TEXT DEFAULT (datetime('now'))
);"

LAST_CLIP=""
echo "Watching clipboard... (Ctrl+C to stop)"

while true; do
    CLIP=$(dcp query clipboard --format=json 2>/dev/null | jq -r '.clipboard.content' 2>/dev/null)
    if [ -n "$CLIP" ] && [ "$CLIP" != "$LAST_CLIP" ] && [ "$CLIP" != "null" ]; then
        APP=$(dcp query activeWindow --format=json 2>/dev/null | jq -r '.activeWindow.application' 2>/dev/null || echo "unknown")
        # Escape single quotes for SQL
        ESCAPED=$(echo "$CLIP" | sed "s/'/''/g")
        ESCAPED_APP=$(echo "$APP" | sed "s/'/''/g")
        sqlite3 "$DB" "INSERT INTO history (content, app) VALUES ('$ESCAPED', '$ESCAPED_APP');"
        echo "$(date): saved clipboard from $APP"
        LAST_CLIP="$CLIP"
    fi
    sleep 1
done
