#!/bin/bash
# status_dashboard.sh - Generate STATUS.md cross-user dashboard for sqlrustgo runs.
#
# Reads the registry + active_soaks output and writes a single STATUS.md
# that any user can read (no root needed) to see what's running, what
# resources are committed, and which port + data_dir to use.
#
# Used by:
#   - humans: `cat test_results/STATUS.md`
#   - cron: every 60s to refresh the dashboard
#   - 兄弟 agents: before launching new runs, see what is in flight
#
# Design (P1 enhancement 2026-06-14):
#   - Read-only input (registry + /proc); write-only to STATUS.md
#   - Atomic write: STATUS.md.tmp → mv (no partial reads)
#   - Stable section ordering: header / registry / host resources / recent finishes
#   - Truncates at 200 lines so it stays scannable

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
REGISTRY="${PROJECT_ROOT}/scripts/stability/run_registry.sh"
ACTIVE_SOAKS="${PROJECT_ROOT}/scripts/monitor/active_soaks.sh"

# Allow override (e.g. when generated into test_results/STATUS.md)
OUTPUT="${1:-${PROJECT_ROOT}/test_results/STATUS.md}"
mkdir -p "$(dirname "$OUTPUT")"

# Disable pipefail around the section calls: `bash <script> | head -N` can
# exit non-zero (head closes pipe early) which would otherwise kill us
# under `set -e`.
set +o pipefail
{
    printf '# sqlrustgo Soak Status Dashboard\n\n'
    printf '_Generated %s by %s@%s_\n\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(id -un)" "$(hostname -s)"

    # ─────────────────────────────────────────────────────────────────────
    # Section 2: registry (port + data_dir occupied)
    # ─────────────────────────────────────────────────────────────────────
    printf '## Active Runs (registry)\n\n'
    if [ -x "$REGISTRY" ]; then
        bash "$REGISTRY" list 2>/dev/null | head -20 || printf '_(registry list failed)_\n'
    else
        printf '_(registry not found at %s)_\n' "$REGISTRY"
    fi
    printf '\n'

    # ─────────────────────────────────────────────────────────────────────
    # Section 3: live processes (cross-user, no root)
    # ─────────────────────────────────────────────────────────────────────
    printf '## Live Processes (cross-user)\n\n'
    if [ -x "$ACTIVE_SOAKS" ]; then
        bash "$ACTIVE_SOAKS" 2>/dev/null | head -20 || printf '_(active_soaks failed)_\n'
    else
        printf '_(active_soaks.sh not found)_\n'
    fi
    printf '\n'

    # ─────────────────────────────────────────────────────────────────────
    # Section 4: host resource summary
    # ─────────────────────────────────────────────────────────────────────
    printf '## Host Resources\n\n'
    if command -v free >/dev/null 2>&1; then
        free -h | head -3
    fi
    printf '\n'
    if command -v uptime >/dev/null 2>&1; then
        uptime
    fi
    printf '\n'

    # ─────────────────────────────────────────────────────────────────────
    # Section 5: recent stability reports
    # ─────────────────────────────────────────────────────────────────────
    printf '## Recent Stability Reports (last 24h)\n\n'
    results_dir="${PROJECT_ROOT}/test_results"
    if [ -d "$results_dir" ]; then
        find "$results_dir" -name 'STABILITY_REPORT.md' -mmin -1440 2>/dev/null | head -5 | while read -r f; do
            printf -- '- `%s`\n' "${f#$PROJECT_ROOT/}"
        done
    else
        printf '_(no test_results/ yet)_\n'
    fi
    printf '\n'

    # ─────────────────────────────────────────────────────────────────────
    # Section 6: footer / tip
    # ─────────────────────────────────────────────────────────────────────
    printf '## Next Steps\n\n'
    printf -- '- Launch a 5-min smoke: `bash scripts/stability/run_soak.sh 5min`\n'
    printf -- '- Launch a 30-min TPC-H: `bash scripts/stability/run_soak.sh tpch`\n'
    printf -- '- Launch a wired 24h: `bash scripts/stability/run_soak.sh wired 24`\n'
    printf -- '- Live watch: `bash scripts/stability/run_soak.sh monitor --watch 10`\n'
    printf -- '- List registry: `bash scripts/stability/run_registry.sh list`\n'
    printf '\n_Dashboard generated every 60s by status_dashboard.sh cron job._\n'
} > "${OUTPUT}.tmp"

# Atomic write
mv "${OUTPUT}.tmp" "$OUTPUT"

# Truncate to 200 lines if too long (keep header intact)
if [ "$(wc -l < "$OUTPUT")" -gt 250 ]; then
    head -200 "$OUTPUT" > "$OUTPUT.tmp"
    printf '_...truncated at 200 lines. See full registry for details._\n' >> "$OUTPUT.tmp"
    mv "$OUTPUT.tmp" "$OUTPUT"
fi

echo "  STATUS.md updated: $OUTPUT ($(wc -l < "$OUTPUT") lines)"
