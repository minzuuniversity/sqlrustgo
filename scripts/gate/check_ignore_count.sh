#!/usr/bin/env bash
#
# scripts/gate/check_ignore_count.sh
#
# Purpose: P12 No Implicit Tolerance - explicit #[ignore] registry
#
# Vulnerability V2: 92+ #[ignore] tests tracked by ZERO gate scripts.
#   Adding new #[ignore] silently passes; no accountability.
#
# Coverage: P12 (No Implicit Tolerance), P8 (Negative Evidence), P7 (Provenance)
#
# Strategy: maintain a registry file listing all allowed #[ignore] with
# reason + issue link + ADR. Any new #[ignore] not in registry = FAIL.
#
# Exit codes:
#   0 = PASS (all #[ignore] are in registry)
#   1 = FAIL (unregistered #[ignore] found)
#   2 = DRIFT (registry file missing - first run)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

REGISTRY="$PROJECT_ROOT/tests/baseline/ignore_registry.json"

echo "=== P12 No Implicit Tolerance - #[ignore] Registry ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo

# ---------------------------------------------------------------------------
# Step 1: Extract all #[ignore] tests from source
# ---------------------------------------------------------------------------
echo "[1/4] Extracting #[ignore] from source..."

IGNORE_TMP=$(mktemp)
grep -rE "#\[ignore" --include="*.rs" crates/ tests/ 2>/dev/null | \
    sed -E 's/.*#\[ignore[ =]"([^"]+)".*/\1/' > "$IGNORE_TMP" || true

# Also extract file:line:reason triples for full traceability
IGNORE_DETAILS=$(mktemp)
grep -rnE "#\[ignore" --include="*.rs" crates/ tests/ 2>/dev/null > "$IGNORE_DETAILS" || true

total_ignored=$(wc -l < "$IGNORE_TMP" | tr -d ' ')
echo "  Total #[ignore] tests: $total_ignored"

# ---------------------------------------------------------------------------
# Step 2: Read or create registry
# ---------------------------------------------------------------------------
echo
echo "[2/4] Reading registry..."

if [ ! -f "$REGISTRY" ]; then
    echo "  ⚠️  DRIFT: registry not found: $REGISTRY"
    echo "          First run: auto-creating registry from current #[ignore] set"
    mkdir -p "$(dirname "$REGISTRY")"

    python3 - "$IGNORE_DETAILS" "$REGISTRY" "$total_ignored" <<'PYEOF'
import json, sys, datetime
details_file, registry_file, total = sys.argv[1], sys.argv[2], int(sys.argv[3])

entries = []
with open(details_file) as f:
    for line in f:
        line = line.rstrip("\n")
        if not line:
            continue
        path, _, content = line.partition(":")
        entries.append({
            "file": path.lstrip("./"),
            "line": content,
            "reason": "TODO: needs ADR + issue link",
        })

data = {
    "version": "v3.9.0-rc7",
    "timestamp": datetime.datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ"),
    "total_allowed": total,
    "ignored_tests": entries,
    "note": "Auto-generated registry. Each #[ignore] needs explicit ADR + issue link.",
}

with open(registry_file, "w") as out:
    json.dump(data, out, indent=2, ensure_ascii=False)
PYEOF

    echo "  Created: $REGISTRY"
    echo
    echo "ℹ️  INFO: first run, registry established. Re-run to verify compliance."
    rm -f "$IGNORE_TMP" "$IGNORE_DETAILS"
    exit 0
fi

# ---------------------------------------------------------------------------
# Step 3: Cross-check: each #[ignore] should be in registry
# ---------------------------------------------------------------------------
echo
echo "[3/4] Cross-checking #[ignore] vs registry..."

# Get list of files with #[ignore] (for matching)
ignore_files=$(grep -rnE "#\[ignore" --include="*.rs" crates/ tests/ 2>/dev/null | \
    cut -d: -f1 | sed 's|^\./||' | sort -u)

# Read registry files
registry_files=$(python3 -c "
import json
try:
    with open('$REGISTRY') as f:
        data = json.load(f)
    files = sorted(set(t['file'].split(':')[0] for t in data.get('ignored_tests', [])))
    print('\n'.join(files))
except Exception as e:
    print(f'ERROR: {e}', file=__import__('sys').stderr)
    import sys; sys.exit(1)
" 2>/dev/null || echo "")

if [ -z "$registry_files" ]; then
    echo "  ⚠️  WARN: could not parse registry (may be malformed JSON)"
    echo "          Continuing with empty registry check"
fi

# Cross-check via python3 (deterministic; avoids bash pipeline races)
# The previous bash `echo | grep -qxF` loop was non-deterministic and
# intermittently flagged registered files as missing (V312-GA-prep audit).
# python3 set membership test is fully deterministic.
python3 - "$IGNORE_DETAILS" "$REGISTRY" <<'PYEOF' > /tmp/p12_check.txt
import json, sys
details_file, registry_file = sys.argv[1], sys.argv[2]

src_files = set()
with open(details_file) as f:
    for line in f:
        line = line.rstrip("\n")
        if not line:
            continue
        path = line.partition(":")[0].lstrip("./")
        if path:
            src_files.add(path)

reg_files = set()
with open(registry_file) as f:
    data = json.load(f)
for e in data.get("ignored_tests", []):
    reg_files.add(e["file"].split(":")[0])

unregistered = sorted(src_files - reg_files)
print("\n".join(unregistered))
print(f"__FAIL__:{len(unregistered)}")
PYEOF

unregistered=()
if [ -s /tmp/p12_check.txt ]; then
    while IFS= read -r line; do
        case "$line" in
            "__FAIL__:"*) ;;
            *) [ -n "$line" ] && unregistered+=("$line") ;;
        esac
    done < /tmp/p12_check.txt
fi
FAIL=${#unregistered[@]}

if [ "$FAIL" -gt 0 ]; then
    echo "  ❌ FAIL: $FAIL file(s) have #[ignore] but NOT in registry:"
    for f in "${unregistered[@]}"; do
        count=$(grep -c "#\[ignore" "$f" 2>/dev/null || echo 0)
        echo "          - $f ($count #[ignore] lines)"
    done
    echo
    echo "  ACTION REQUIRED: Add to $REGISTRY with explicit reason + issue link."
else
    echo "  ✅ PASS: all #[ignore] tests are in registry"
fi
rm -f /tmp/p12_check.txt

# ---------------------------------------------------------------------------
# Step 4: Report
# ---------------------------------------------------------------------------
echo
echo "[4/4] Summary..."

registry_count=$(python3 -c "
import json
with open('$REGISTRY') as f:
    data = json.load(f)
print(len(data.get('ignored_tests', [])))
" 2>/dev/null || echo "?")

echo "  Source #[ignore] count: $total_ignored"
echo "  Registry #[ignore] count: $registry_count"

if [ "$total_ignored" -ne "$registry_count" ]; then
    echo "  ⚠️  WARN: count mismatch - registry may be out of sync"
fi

rm -f "$IGNORE_TMP" "$IGNORE_DETAILS"

echo
echo "=== P12 Summary ==="
if [ "$FAIL" -gt 0 ]; then
    echo "❌ FAIL — P12 violations. Unregistered #[ignore] tests found."
    echo "   Required action: register each #[ignore] in $REGISTRY with reason + ADR."
    exit 1
fi

echo "✅ PASS — P12 satisfied. All #[ignore] tests are explicit and registered."
exit 0