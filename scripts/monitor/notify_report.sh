#!/bin/bash
# notify_report.sh - Email/webhook notification when a stability run completes.
#
# Designed to be called as a final step from run_*.sh, OR from a watcher
# that detects new STABILITY_REPORT.md files.
#
# Channels (tried in order, skipped on failure):
#   1. Local mail (sendmail/mail) — for mail configured locally
#   2. Slack/Discord webhook (if WEBHOOK_URL set)
#   3. Desktop notification (osascript on macOS, notify-send on Linux)
#   4. Just append to a log file
#
# Configuration via env:
#   NOTIFY_EMAIL=alice@example.com,bob@example.com
#   NOTIFY_WEBHOOK_URL=https://hooks.slack.com/services/...
#   NOTIFY_DRY_RUN=1   # print, don't actually send
#
# Usage:
#   notify_report.sh <STABILITY_REPORT.md>
#   notify_report.sh <STABILITY_REPORT.md> PASS     # explicit verdict
#   notify_report.sh --watch DIR                   # monitor a dir for new reports
#
# P2-2 enhancement (after P0/P1 verified, 2026-06-14).

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

CMD="${1:-}"

# If first arg is --watch, run a watcher loop
if [ "$CMD" = "--watch" ]; then
    WATCH_DIR="${2:?watch dir required}"
    shift 2
    if [ ! -d "$WATCH_DIR" ]; then
        echo "FAIL: $WATCH_DIR not a dir" >&2
        exit 1
    fi
    # Track already-seen files
    declare -A seen
    while true; do
        for f in "$WATCH_DIR"/*/STABILITY_REPORT.md; do
            [ -f "$f" ] || continue
            [ -n "${seen[$f]:-}" ] && continue
            # Skip if file mtime is older than 5 min (avoid re-notifying)
            if [ "$(( $(date +%s) - $(stat -c %Y "$f" 2>/dev/null || stat -f %m "$f") ))" -lt 300 ]; then
                seen[$f]=1
                bash "$0" "$f" &
            fi
        done
        sleep 30
    done
    exit 0
fi

REPORT="$CMD"
if [ -z "$REPORT" ] || [ ! -f "$REPORT" ]; then
    echo "Usage: $0 <STABILITY_REPORT.md> [verdict]" >&2
    echo "       $0 --watch <dir>" >&2
    exit 1
fi

VERDICT="${2:-}"
if [ -z "$VERDICT" ]; then
    # Extract from report
    VERDICT=$(grep -oE '\*\*(PASS|FAIL|NEEDS REVIEW)\*\*' "$REPORT" | head -1 | tr -d '*')
fi
[ -z "$VERDICT" ] && VERDICT="UNKNOWN"

HOSTNAME_S=$(hostname -s)
SUBJECT="[sqlrustgo/${HOSTNAME_S}] Stability run ${VERDICT}: $(basename "$(dirname "$REPORT")")"

# Build short body (first 80 lines)
BODY=$(head -80 "$REPORT")
FULL_PATH="$REPORT"

notify_log() {
    local msg="[$(date -u +%Y-%m-%dT%H:%M:%SZ)] $SUBJECT
$BODY
---
"
    local log_file="${NOTIFY_LOG:-$HOME/.sqlrustgo-cron/notifications.log}"
    mkdir -p "$(dirname "$log_file")"
    echo "$msg" >> "$log_file"
    echo "  [notify] logged to $log_file"
}

notify_email() {
    local emails="${NOTIFY_EMAIL:-}"
    [ -z "$emails" ] && { echo "  [notify] no NOTIFY_EMAIL set, skipping"; return 0; }
    local cmd
    cmd=$(command -v mail || command -v sendmail || true)
    if [ -z "$cmd" ]; then
        echo "  [notify] no mail/sendmail binary, skipping"
        return 1
    fi
    if [ "$NOTIFY_DRY_RUN" = "1" ]; then
        echo "  [notify] DRY: would send to: $emails"
        return 0
    fi
    # shellcheck disable=SC2086
    printf '%s\n\nFull report: %s\n' "$BODY" "$FULL_PATH" | $cmd $emails
    echo "  [notify] email sent via $cmd to: $emails"
}

notify_webhook() {
    local url="${NOTIFY_WEBHOOK_URL:-}"
    [ -z "$url" ] && { echo "  [notify] no NOTIFY_WEBHOOK_URL, skipping"; return 0; }
    if [ "$NOTIFY_DRY_RUN" = "1" ]; then
        echo "  [notify] DRY: would POST to: $url"
        return 0
    fi
    # Slack-compatible payload (also works for Discord with minor edit)
    local payload
    payload=$(printf '{"text":"%s\n\n```\n%s\n```\n"}' "$SUBJECT" "$BODY" | python3 -c '
import json, sys
text = sys.stdin.read()
# Escape for JSON
text = text.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n")
print(json.dumps({"text": text}))
' 2>/dev/null || echo "{\"text\":\"$SUBJECT\"}")
    curl -sS --max-time 10 -X POST -H 'Content-Type: application/json' -d "$payload" "$url" >/dev/null 2>&1
    echo "  [notify] webhook posted to: $url"
}

notify_desktop() {
    # macOS
    if command -v osascript >/dev/null 2>&1; then
        osascript -e "display notification \"$SUBJECT\" with title \"sqlrustgo stability\"" 2>/dev/null
        echo "  [notify] macOS notification sent"
        return
    fi
    # Linux
    if command -v notify-send >/dev/null 2>&1; then
        notify-send "$SUBJECT" "$BODY" 2>/dev/null
        echo "  [notify] Linux notification sent"
        return
    fi
    echo "  [notify] no desktop notification tool"
}

echo "=== Stability Run Notification ==="
echo "Subject: $SUBJECT"
echo "Report: $FULL_PATH"
echo "Verdict: $VERDICT"
echo ""

notify_log
notify_email
notify_webhook
notify_desktop

echo ""
echo "Done."
