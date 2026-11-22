#!/usr/bin/env bash
#
# scripts/gate/check_drift_not_pass.sh
#
# Purpose: P14 DRIFT != PASS - detect gates that treat DRIFT (exit 2) as PASS
#
# Vulnerability V5: check_full_gate_verification.sh's run_gate() function:
#   elif [ "$code" -eq 2 ] && [ "$expect_code" -eq 0 ]; then
#       echo "  ⚠️  DRIFT (exit 2, expected 0)"
#       DRIFT_COUNT=$((DRIFT_COUNT+1))
#   ...
# DRIFT is logged but final summary treats it as not-a-blocker.
# This violates P14 (DRIFT must NOT silently pass).
#
# Coverage: P14 (DRIFT != PASS), P5 (Governance > Features)
#
# Strategy: scan all gate scripts for anti-patterns that:
#   1. Accept exit 2 as success
#   2. Use `|| true` to swallow exit codes
#   3. Return 0 from a function/branch that includes DRIFT handling
#
# Exit codes:
#   0 = PASS (no DRIFT-accepting patterns found)
#   1 = FAIL (anti-patterns found)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

echo "=== P14 DRIFT != PASS - Anti-pattern Detector ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo

FAIL=0
FINDINGS=()

# How many gate scripts were actually scanned? A gate that scans zero
# scripts finds zero anti-patterns and says PASS — that is a vacuous green,
# not a verification. The count is asserted below.
SCANNED_GATE_SCRIPTS=0

# ---------------------------------------------------------------------------
# Fail-closed: a missing baseline must NOT be self-healing.
#
# The previous shape auto-created the baseline on first run and then reset
# FAIL=0, so the gate exited 0 (PASS) **on a tree where it had just found
# anti-patterns**. Measured on the authoritative tree (2026-11-23,
# 8cbde5b6d3): with `tests/baseline/drift_baseline.json` absent the gate
# reported 14 findings — including "check_full_gate_verification.sh accepts
# exit 2 (DRIFT) without proper failure handling", which is the exact V5
# vulnerability this gate exists to detect — and then printed
# "✅ PASS — P14 satisfied" with rc=0.
#
# A gate that manufactures its own reference data mid-verification asserts
# nothing: the baseline it writes contains exactly the debt it just found,
# so the second run is green and the first run was a false green too.
# The sibling gate `check_gate_test_integrity.sh` was already fixed for the
# identical defect (#5113: "a missing baseline is a FAIL, not a 'first run,
# capturing it'"). This gate had the same hole and the same fix was owed.
#
# Recording the baseline stays possible, but it becomes an explicit,
# reviewable act: `--init-baseline`.
# ---------------------------------------------------------------------------
INIT_BASELINE=0
for arg in "$@"; do
    case "$arg" in
        --init-baseline) INIT_BASELINE=1 ;;
        *) : ;;
    esac
done

# ---------------------------------------------------------------------------
# Check 1: gate scripts that accept exit 2 as success
# ---------------------------------------------------------------------------
echo "[1/5] Scanning for DRIFT-accepting patterns..."

# B fix: grep for DRIFT-handling in gate scripts, excluding P14 self (self-reference is by design)
# 2026-11-08（Owner 批准 GA-DRIFT2 判据修改）：`scripts/gate/legacy/` 装的是**历史实现**
# （不在 168 分母内、不由任何清单/归档命令集调用），把它当现行门禁扫 ⇒ 判据陈旧 ⇒ 假红。
# 显式排除，并在下面把排除原因写进判据说明（判据口径必须自解释）。
drift_pattern=$(grep -rnE '\[\s*"\$?(code|exit_code|status)"?\s*-eq\s*2\s*\]' \
    scripts/gate/ --exclude='check_drift_not_pass.sh' --exclude-dir='legacy' 2>/dev/null | head -20)

# Count the gate scripts this run actually polices. The two detectors are
# excluded because the scan excludes them too (self-reference is by design):
# a watchdog that watches no other gate has nothing to say.
SCANNED_GATE_SCRIPTS=$(find "$SCRIPT_DIR" -maxdepth 1 -name '*.sh' -type f \
    ! -path "$SCRIPT_DIR/legacy/*" \
    ! -name 'check_drift_not_pass.sh' \
    ! -name 'check_gate_test_integrity.sh' \
    2>/dev/null | wc -l | tr -d ' ')

if [ -n "$drift_pattern" ]; then
    echo "  Found $(echo "$drift_pattern" | wc -l | tr -d ' ') potential DRIFT-handling sites"
    while IFS=: read -r filepath linenum content; do
        [ -z "$filepath" ] && continue

        # Check if this site accepts DRIFT (no exit 1 nearby)
        # Look at next 5 lines for exit 1
        next_5=$(sed -n "$((linenum+1)),$((linenum+5))p" "$filepath" 2>/dev/null)
        if ! echo "$next_5" | grep -qE 'exit [1-9]|FAIL|return [1-9]|BLOCKERS'; then
            FINDINGS+=("$filepath:$linenum: accepts exit 2 without explicit FAIL")
            FAIL=$((FAIL+1))
            echo "    ❌ $filepath:$linenum - accepts exit 2 without exit 1"
        fi
    done <<< "$drift_pattern"
fi

# ---------------------------------------------------------------------------
# Check 2: `|| true` to swallow exit codes (V6)
# ---------------------------------------------------------------------------
echo
echo "[2/5] Scanning for '|| true' patterns that swallow errors..."

# The scan strips trailing comments first, exactly like the P16 detector in
# check_gate_test_integrity.sh (its V312-31 refinement). Without that, the
# *documentation* of an anti-pattern — e.g. a comment quoting
# `cargo test ... || true` — is reported as the anti-pattern itself, and
# the baseline has to absorb the false positives instead of the real ones.
swallow_pattern=$(find "$SCRIPT_DIR" -name '*.sh' -type f ! -path "$SCRIPT_DIR/legacy/*" -print0 2>/dev/null \
| while IFS= read -r -d '' f; do
    sed 's/[[:space:]]*#.*$//' "$f" \
    | grep -nE 'cargo [a-z]+ [^|]+\|\|\s*true|>\s*/[^[:space:]]+\s*2>&1\s*\|\|\s*true' \
    | grep -vE '^[[:space:]]*[0-9]+:[[:space:]]*(step|pass|fail|warn|echo|printf)[[:space:]]' \
    | while IFS= read -r hit; do
        echo "$f:${hit%%:*}:$(echo "$hit" | cut -d: -f2-)"
    done
done | head -20)

if [ -n "$swallow_pattern" ]; then
    echo "  Found $(echo "$swallow_pattern" | wc -l | tr -d ' ') sites using '|| true' or swallow patterns"
    while IFS=: read -r filepath linenum content; do
        [ -z "$filepath" ] && continue
        FINDINGS+=("$filepath:$linenum: uses '|| true' to swallow exit codes")
        echo "    ⚠️  $filepath:$linenum"
    done <<< "$swallow_pattern"
fi

# ---------------------------------------------------------------------------
# Check 3: gate scripts that use `grep` on test output without verifying pattern
# ---------------------------------------------------------------------------
echo
echo "[3/5] Scanning for grep-on-stdout without result validation..."

# #5403: this used to end in `| head -10`, which truncated the scan to the
# first ten matches **in filesystem order**. That made the gate's verdict
# depend on which files the filesystem happened to list first: on a
# developer machine the ten it saw were all already in the baseline (so it
# reported "no NEW anti-patterns" and passed), while on CI the order put
# `check_p21_audit_log.sh`, `check_p13_soak_test.sh` and `check_p35_simd.sh`
# inside the first ten, none of which were registered, so the same tree was
# reported as a P0 hard-block failure.
#
# A gate whose result changes with directory listing order is not a gate.
# The scan now covers **every** match; the baseline records the real debt
# (24 sites), and any new one is still a failure.
grep_pattern=$(grep -rnE 'cargo test[^|]+\|[^|]*grep' scripts/gate/ --exclude-dir='legacy' 2>/dev/null || true)

if [ -n "$grep_pattern" ]; then
    echo "  Found $(echo "$grep_pattern" | wc -l | tr -d ' ') grep-on-stdout sites"
    while IFS=: read -r filepath linenum content; do
        [ -z "$filepath" ] && continue

        # Check if exit code is also checked (defense in depth)
        if ! grep -q "PIPESTATUS\|\\\$?" "$filepath" 2>/dev/null; then
            FINDINGS+=("$filepath:$linenum: grep on cargo test stdout without PIPESTATUS check")
            FAIL=$((FAIL+1))
            echo "    ❌ $filepath:$linenum"
        fi
    done <<< "$grep_pattern"
fi

# ---------------------------------------------------------------------------
# Check 4: Verify run_gate() in check_full_gate_verification.sh specifically
# ---------------------------------------------------------------------------
echo
echo "[4/5] Specific check: check_full_gate_verification.sh run_gate()"

full_gate_script="$SCRIPT_DIR/check_full_gate_verification.sh"
if [ -f "$full_gate_script" ]; then
    # Look for run_gate function and its DRIFT handling
    # Check if DRIFT pattern exists AND is followed by proper failure handling
    drtift_line=$(grep -nE '\[\s*"\$?code"?\s*-eq\s*2\s*\]' "$full_gate_script" 2>/dev/null | head -1)
    if [ -n "$drtift_line" ]; then
        linenum=$(echo "$drtift_line" | cut -d: -f1)
        # Check next 10 lines for proper DRIFT handling (return 1 or exit 1)
        next_10=$(sed -n "$((linenum)),$((linenum+10))p" "$full_gate_script" 2>/dev/null)
        if echo "$next_10" | grep -qE 'return\s+1|exit\s+1'; then
            echo "  ✅ PASS: run_gate() properly handles DRIFT with failure code"
        else
            echo "  ❌ FAIL: $full_gate_script accepts exit 2 (DRIFT) without proper failure handling"
            FINDINGS+=("$full_gate_script: run_gate() accepts DRIFT as PASS")
            FAIL=$((FAIL+1))
        fi
    else
        echo "  ✅ PASS: run_gate() no longer has DRIFT pattern"
    fi
else
    echo "  ⚠️  INFO: $full_gate_script not found (D-gate may be inactive)"
fi

# ---------------------------------------------------------------------------
# Check 5 (#5426): a swallowed exit code that is then read
# ---------------------------------------------------------------------------
# Checks 2 and 4 only looked for `cargo ... || true`. They therefore
# missed the #5417 shape:
#
#     output=$(bash "$script" 2>&1 || true)
#     local code=$?
#
# `|| true` forces the command substitution to exit 0, so `$?` is
# permanently 0 and the gate reports PASS for a failing sub-gate. The
# same shape appeared twice more in check_rc_ga_gate.sh (B5 integration
# gate, D4 WAL invariants).
#
# The general form is: a `|| true` whose exit status is read on the
# next line. That is what this check detects. It is deliberately narrow
# — a `|| true` that is NOT followed by an exit-status read is a
# different (and usually benign) pattern, e.g. a log redirection whose
# failure is irrelevant.
echo ""
echo "[5/5] Scanning for '|| true' whose exit status is read afterwards..."

# The two detectors are excluded: their own source contains the pattern
# they look for (self-reference is by design, same as Check 1).
SWALLOW_READ=$(find "$SCRIPT_DIR" -name '*.sh' -type f \
    ! -path "$SCRIPT_DIR/legacy/*" \
    ! -name 'check_drift_not_pass.sh' \
    ! -name 'check_gate_test_integrity.sh' \
    -print0 2>/dev/null \
| while IFS= read -r -d '' f; do
    awk -v file="$f" '
        {
            line = $0
            sub(/^[ \t]+/, "", line)
            # comment lines are documentation, not code
            if (substr(line, 1, 1) == "#") { was_or_true = 0; next }
            if (was_or_true && index(line, "$?") > 0) {
                printf "%s:%d: `|| true` swallows the exit code, which is read on the next line\n", file, NR
            }
            was_or_true = (line ~ /\|\|[[:space:]]*true/)
        }
    ' "$f"
done)

SWALLOW_READ_COUNT=0
if [ -n "$SWALLOW_READ" ]; then
    while IFS= read -r site; do
        [ -z "$site" ] && continue
        FINDINGS+=("$site")
        FAIL=$((FAIL+1))
        SWALLOW_READ_COUNT=$((SWALLOW_READ_COUNT + 1))
        echo "    ❌ $site"
    done <<< "$SWALLOW_READ"
    echo "  ❌ $SWALLOW_READ_COUNT site(s) where a swallowed exit code is read afterwards"
else
    echo "  ✅ PASS: no swallowed exit code is read afterwards"
fi

# ---------------------------------------------------------------------------
# Baseline regression detection
# ---------------------------------------------------------------------------
DRIFT_BASELINE="$PROJECT_ROOT/tests/baseline/drift_baseline.json"
mkdir -p "$(dirname "$DRIFT_BASELINE")"

if [ ! -f "$DRIFT_BASELINE" ]; then
    if [ "$INIT_BASELINE" != "1" ]; then
        # Fail closed: no reference data ⇒ nothing was compared ⇒ a PASS
        # here would assert nothing.
        echo
        echo "  ❌ FAIL: DRIFT anti-pattern baseline not found: $DRIFT_BASELINE"
        echo "     A missing baseline means P14 has no reference to compare against,"
        echo "     so a PASS would assert nothing — and self-creating the baseline"
        echo "     here would silently absorb whatever debt this run just found"
        echo "     (measured: 14 findings absorbed, rc=0, 'P14 satisfied')."
        echo "     Record the baseline deliberately with --init-baseline, then commit it."
        exit 1
    fi
    echo
    echo "  ℹ️  --init-baseline given: recording the DRIFT-anti-pattern baseline..."
    {
        echo "{"
        echo "  \"created_at\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
        # #5473: this used to write the **counter** $FAIL, which is not the
        # number of entries in the list below it — the counter is bumped once
        # per *check*, while the list holds one entry per *finding*. The
        # baseline therefore claimed 22 findings and listed 29. Nothing read
        # the field, so the mismatch was invisible until a human did.
        # It is now the length of the list, which is what it says it is.
        echo "  \"total_findings\": ${#FINDINGS[@]},"
        echo "  \"findings\": ["
        first=1
        for f in "${FINDINGS[@]}"; do
            [ $first -eq 0 ] && echo ","
            printf "    %s" "$(python3 -c "import json,sys; print(json.dumps(sys.argv[1]))" "$f")"
            first=0
        done
        echo
        echo "  ]"
        echo "}"
    } > "$DRIFT_BASELINE"
    echo "  Created: $DRIFT_BASELINE"
    echo "  ℹ️  Re-run to verify no NEW anti-patterns."
    FAIL=0
else
    BASELINE_FINDINGS=$(python3 -c "
import json
with open('$DRIFT_BASELINE') as f:
    data = json.load(f)
for finding in data.get('findings', []):
    print(finding)
" 2>/dev/null)

    NEW_FINDINGS=()
    for f in "${FINDINGS[@]}"; do
        # B fix: compare by basename to be portable across absolute paths
        f_basename=$(basename "$f" 2>/dev/null || echo "$f")
        is_known=0
        while IFS= read -r baseline_f; do
            [ -z "$baseline_f" ] && continue
            baseline_basename=$(basename "$baseline_f" 2>/dev/null || echo "$baseline_f")
            if [ "$f" = "$baseline_f" ] || [ "$f_basename" = "$baseline_basename" ]; then
                is_known=1
                break
            fi
        done <<< "$BASELINE_FINDINGS"
        if [ "$is_known" -eq 0 ]; then
            NEW_FINDINGS+=("$f")
        fi
    done

    if [ ${#NEW_FINDINGS[@]} -gt 0 ]; then
        echo
        echo "  ⚠️  REGRESSION: ${#NEW_FINDINGS[@]} NEW anti-pattern(s) (not in baseline):"
        for f in "${NEW_FINDINGS[@]}"; do
            echo "    - $f"
        done
        FINDINGS=("${NEW_FINDINGS[@]}")
        FAIL=${#NEW_FINDINGS[@]}
    else
        echo
        echo "  ✅ No NEW anti-patterns vs baseline"
        FINDINGS=()
        FAIL=0
    fi
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
# ---------------------------------------------------------------------------
# Zero-execution guard: a scan that looked at nothing must not report PASS.
# ---------------------------------------------------------------------------
if [ "$SCANNED_GATE_SCRIPTS" -eq 0 ]; then
    echo
    echo "  ❌ FAIL: 0 gate scripts were scanned — the DRIFT scan executed nothing,"
    echo "     so a PASS here would assert nothing. (check scripts/gate/ exists and"
    echo "     is populated; a gate that scans an empty tree is a vacuous green.)"
    exit 1
fi

echo
echo "=== P14 Summary ==="
echo "  Total anti-patterns: $FAIL"

if [ "$FAIL" -gt 0 ]; then
    echo
    echo "  Specific findings:"
    for f in "${FINDINGS[@]}"; do
        echo "    - $f"
    done
    echo
    echo "❌ FAIL — P14 violations. DRIFT-accepting patterns found in gate scripts."
    echo "   Required action: fix gate scripts to exit 1 on DRIFT (not exit 0)."
    exit 1
fi

echo
echo "✅ PASS — P14 satisfied. No gate script treats DRIFT as PASS."
exit 0