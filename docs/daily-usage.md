# Using DCP Day-to-Day

## Quick Reference

```bash
# What's on my screen?
dcp workflow whereami

# What did I just copy?
dcp workflow clip

# System resources
dcp workflow sys

# Track focus throughout the day
scripts/dcp-focus-tracker.sh &

# Meeting mode (auto DND)
scripts/dcp-meeting-mode.sh start

# Clipboard history
scripts/dcp-clipboard-history.sh &

# Health check (add to crontab)
*/30 * * * * ~/.local/bin/dcp-health.sh
```

## Integration with AI Assistants

### Claude Code / aider
```bash
# Ask Claude what you're working on
dcp query activeWindow | jq -r '.activeWindow.title'
```

### Open WebUI
Add the DCP tool to Open WebUI for desktop-aware AI:
`integrations/open-webui/dcp-tool.py`

### Custom AI agent
```python
from dcp_client import DcpClient
import json

async with DcpClient() as client:
    ctx = await client.query("activeWindow", "clipboard", "runningProcesses")
    print(f"You are working on: {ctx.active_window.title}")
    print(f"Clipboard: {ctx.clipboard.content[:100]}")
```

## Automation Ideas
- **Auto-backup**: When focus is on your project dir, auto-commit
- **Time tracking**: Log active window to a timesheet
- **Context-aware AI**: Feed desktop context to LLM prompts
- **Smart DND**: Mute notifications during fullscreen/coding
- **Clipboard manager**: Searchable history of copied items
