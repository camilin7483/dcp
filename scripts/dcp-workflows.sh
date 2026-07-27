#!/bin/bash
# DCP Workflow Manager — utility scripts for common automations
# Source this file in your .zshrc or .bashrc

dcp-workflow() {
    case "${1:-help}" in
        status)
            dcp status
            ;;
        focus)
            dcp query activeWindow --format=pretty
            ;;
        clip)
            dcp query clipboard --format=pretty
            ;;
        procs)
            dcp query runningProcesses --format=table 2>/dev/null | head -20
            ;;
        sys)
            dcp query systemResources --format=pretty
            ;;
        whereami)
            echo "--- Desktop Context ---"
            dcp query activeWindow systemResources network --format=table
            ;;
        monitor)
            dcp query monitors mouse --format=pretty
            ;;
        notify)
            dcp subscribe notification --format=pretty
            ;;
        health)
            dcp status
            echo "---"
            echo "Plugins:"
            ls -la "$HOME/.local/share/dcpd/plugins/" 2>/dev/null || echo "  no plugins installed"
            echo "Logs:"
            ls -lt "$HOME/.local/share/dcpd/audit/" 2>/dev/null | head -5 || echo "  no audit logs"
            ;;
        help|*)
            echo "DCP Workflow Commands:"
            echo "  status    - Daemon status"
            echo "  focus     - Active window info"
            echo "  clip      - Clipboard content"
            echo "  procs     - Top processes"
            echo "  sys       - System resources"
            echo "  whereami  - Full desktop context"
            echo "  monitor   - Display info"
            echo "  notify    - Listen for notifications"
            echo "  health    - Health check"
            ;;
    esac
}
