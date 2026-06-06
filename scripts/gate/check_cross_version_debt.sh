#!/usr/bin/env bash

# check_cross_version_debt.sh — Cross-Version Debt Gate Check (EXTENDED)
#
# This script checks cross-version debt status for v3.8.0 GA gate.
# Validates:
#   - INT-1~INT-4 (Integration Debt, from CROSS-VERSION-DEBT.md)
#   - F-01~F-36 (Feature Debt, from v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md)
#   - I-01~I-12 (Integration Debt v3.0.0 era, from same source)
#   - T-01~T-20 (Test Debt, from same source)
# Total: 4 + 36 + 12 + 20 = 72 cross-version debt items tracked
#
# Source: docs/releases/v3.8.0/INT5_PLUS_DEBT_INVENTORY.md
#
# Usage: ./scripts/gate/check_cross_version_debt.sh
#        ./scripts/gate/check_cross_version_debt.sh --strict (fail on any ACTIVE/OPEN)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")"
# v3.8.0 PR-2933 reorganized docs into categorized subdirectories.
# Prefer the canonical reorg location (debt/CROSS-VERSION-DEBT.md) which
# contains the formatted status tables. Fall back to legacy root or SPEC.
CROSS_VERSION_DEBT_DOC=""
for candidate in \
    "$REPO_DIR/docs/releases/v3.8.0/CROSS-VERSION-DEBT.md" \
    "$REPO_DIR/docs/releases/v3.8.0/debt/CROSS-VERSION-DEBT.md" \
    "$REPO_DIR/docs/releases/v3.8.0/archived/CROSS-VERSION-DEBT.md" \
    "$REPO_DIR/docs/releases/v3.8.0/specs/gate/SPEC-008-cross-version-debt.md" ; do
    if [ -f "$candidate" ]; then
        CROSS_VERSION_DEBT_DOC="$candidate"
        break
    fi
done
INT5_INVENTORY_DOC=""
for candidate in \
    "$REPO_DIR/docs/releases/v3.8.0/INT5_PLUS_DEBT_INVENTORY.md" \
    "$REPO_DIR/docs/releases/v3.8.0/debt/INT5_PLUS_DEBT_INVENTORY.md" ; do
    if [ -f "$candidate" ]; then
        INT5_INVENTORY_DOC="$candidate"
        break
    fi
done

STRICT_MODE=false
if [ "${1:-}" = "--strict" ]; then
    STRICT_MODE=true
fi

echo "=== Running Cross-Version Debt Gate Check (EXTENDED) ==="
cd "$REPO_DIR"
pwd

# Check that required docs exist
MISSING_DOCS=()
if [ -z "$CROSS_VERSION_DEBT_DOC" ]; then
    MISSING_DOCS+=("$REPO_DIR/docs/releases/v3.8.0/CROSS-VERSION-DEBT.md (or archived/ or specs/gate/SPEC-008-cross-version-debt.md)")
fi
if [ -z "$INT5_INVENTORY_DOC" ]; then
    MISSING_DOCS+=("$REPO_DIR/docs/releases/v3.8.0/INT5_PLUS_DEBT_INVENTORY.md (or debt/)")
fi
if [ ${#MISSING_DOCS[@]} -gt 0 ]; then
    echo "❌ FAIL: Required docs missing:"
    for d in "${MISSING_DOCS[@]}"; do
        echo "    - $d"
    done
    exit 1
fi
echo "✅ Cross-version debt docs resolved:"
echo "    CROSS-VERSION-DEBT: $(realpath --relative-to="$REPO_DIR" "$CROSS_VERSION_DEBT_DOC" 2>/dev/null || echo "$CROSS_VERSION_DEBT_DOC")"
echo "    INT5_INVENTORY:      $(realpath --relative-to="$REPO_DIR" "$INT5_INVENTORY_DOC" 2>/dev/null || echo "$INT5_INVENTORY_DOC")"

declare -A DEBT_STATUS
TOTAL=0
PASS_COUNT=0
FAIL_COUNT=0
WARN_COUNT=0

# ============================================================
# Part 1: INT-1~INT-4 (from CROSS-VERSION-DEBT.md)
# ============================================================
echo ""
echo "=== Part 1: Integration Debt (INT-1~INT-4) ==="

for debt_id in INT-1 INT-2 INT-3 INT-4; do
    TOTAL=$((TOTAL + 1))
    status_line=$(grep -A2 "$debt_id" "$CROSS_VERSION_DEBT_DOC" | grep -E "ACTIVE|CLOSED|DEFERRED" | head -1)
    if echo "$status_line" | grep -q "ACTIVE"; then
        DEBT_STATUS[$debt_id]="ACTIVE"
    elif echo "$status_line" | grep -q "CLOSED"; then
        DEBT_STATUS[$debt_id]="CLOSED"
    elif echo "$status_line" | grep -q "DEFERRED"; then
        DEBT_STATUS[$debt_id]="DEFERRED"
    else
        status_line=$(grep "$debt_id" "$CROSS_VERSION_DEBT_DOC" | grep -E "ACTIVE|CLOSED|DEFERRED" | head -1)
        if echo "$status_line" | grep -q "ACTIVE"; then
            DEBT_STATUS[$debt_id]="ACTIVE"
        elif echo "$status_line" | grep -q "CLOSED"; then
            DEBT_STATUS[$debt_id]="CLOSED"
        elif echo "$status_line" | grep -q "DEFERRED"; then
            DEBT_STATUS[$debt_id]="DEFERRED"
        else
            DEBT_STATUS[$debt_id]="UNKNOWN"
        fi
    fi
    status=${DEBT_STATUS[$debt_id]}
    echo "  $debt_id: $status"
    if [ "$status" = "UNKNOWN" ]; then
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
done

# ============================================================
# Part 2: F-01~F-36 (Feature Debt, from INT5_PLUS_DEBT_INVENTORY.md)
# ============================================================
echo ""
echo "=== Part 2: Feature Debt (F-01~F-36) ==="

# Check inventory table for status
for f_id in $(seq -f "F-%02g" 1 36); do
    TOTAL=$((TOTAL + 1))
    # Look for the F-xx row and check status column
    status_line=$(grep -E "^\| (${f_id}) \||^${f_id} \||\| ${f_id} \|" "$INT5_INVENTORY_DOC" | head -1)
    # Extract status indicator (✅ = closed, ⚠️ = partial, ❌ = open)
    if echo "$status_line" | grep -q "✅"; then
        DEBT_STATUS[$f_id]="CLOSED"
    elif echo "$status_line" | grep -q "⚠️"; then
        DEBT_STATUS[$f_id]="PARTIAL"
    elif echo "$status_line" | grep -q "❌"; then
        DEBT_STATUS[$f_id]="OPEN"
    elif echo "$status_line" | grep -q "DEFERRED"; then
        DEBT_STATUS[$f_id]="DEFERRED"
    else
        DEBT_STATUS[$f_id]="UNKNOWN"
    fi
    status=${DEBT_STATUS[$f_id]}
    if [ "$status" = "UNKNOWN" ]; then
        echo "  ⚠️  $f_id: $status (could not extract from inventory)"
        WARN_COUNT=$((WARN_COUNT + 1))
    else
        echo "  $f_id: $status"
    fi
done

# ============================================================
# Part 3: I-01~I-12 (Integration v3.0.0 era)
# ============================================================
echo ""
echo "=== Part 3: Integration Debt v3.0.0 (I-01~I-12) ==="

for i in $(seq -f "I-%02g" 1 12); do
    TOTAL=$((TOTAL + 1))
    status_line=$(grep -E "^\| (${i}) \||^${i} \||\| ${i} \|" "$INT5_INVENTORY_DOC" | head -1)
    if echo "$status_line" | grep -q "✅"; then
        DEBT_STATUS[$i]="CLOSED"
    elif echo "$status_line" | grep -q "⚠️"; then
        DEBT_STATUS[$i]="PARTIAL"
    elif echo "$status_line" | grep -q "❌"; then
        DEBT_STATUS[$i]="OPEN"
    else
        DEBT_STATUS[$i]="UNKNOWN"
    fi
    status=${DEBT_STATUS[$i]}
    if [ "$status" = "UNKNOWN" ]; then
        echo "  ⚠️  $i: $status"
        WARN_COUNT=$((WARN_COUNT + 1))
    else
        echo "  $i: $status"
    fi
done

# ============================================================
# Part 4: T-01~T-20 (Test Debt)
# ============================================================
echo ""
echo "=== Part 4: Test Debt (T-01~T-20) ==="

for t in $(seq -f "T-%02g" 1 20); do
    TOTAL=$((TOTAL + 1))
    status_line=$(grep -E "^\| (${t}) \||^${t} \||\| ${t} \|" "$INT5_INVENTORY_DOC" | head -1)
    if echo "$status_line" | grep -q "✅"; then
        DEBT_STATUS[$t]="CLOSED"
    elif echo "$status_line" | grep -q "⚠️"; then
        DEBT_STATUS[$t]="PARTIAL"
    elif echo "$status_line" | grep -q "❌"; then
        DEBT_STATUS[$t]="OPEN"
    else
        DEBT_STATUS[$t]="UNKNOWN"
    fi
    status=${DEBT_STATUS[$t]}
    if [ "$status" = "UNKNOWN" ]; then
        echo "  ⚠️  $t: $status"
        WARN_COUNT=$((WARN_COUNT + 1))
    else
        echo "  $t: $status"
    fi
done

# ============================================================
# Part 5: Code Reality Check (added in #3106 / #3136 follow-up scope)
# ============================================================
# Per PR #3097 audit §2.7, the previous parts only parsed markdown
# status symbols. This part adds lightweight code-level checks to
# detect "STALE" claims where the inventory says CLOSED but the
# actual codebase is missing implementation or only has isolated
# self-contained test mocks.
#
# Scope: 10 isolated F-XX (#3102) + 5 unimplemented debt items (#3103).
# Full check (cross-cutting use-statement + main-path analysis) is
# tracked in follow-up #3136 (1 week work).
echo ""
echo "=== Part 5: Code Reality Check (#3102, #3103, #3136) ==="

REALITY_FAIL=0
REALITY_WARN=0

# --- 5a: Detect isolated tests for 10 F-XX (#3102) ---
# A test is "isolated" if it uses self-contained struct definitions
# (via `use super::` or none) instead of importing from production
# crates. We probe the file content for the two strongest signals:
#   (a) `use sqlrustgo_` — strong indicator of crate-internal use
#   (b) zero production-struct constructors in body (heuristic)
ISOLATED_F_LIST=(F-16 F-23 F-24 F-25 F-26 F-27 F-29 F-31 F-32 F-35)
# Hardcoded mapping: F-id -> test file basename (per docs/releases/v3.8.0/historical/LEGACY_ISSUES_2026-06-05_AUDIT.md §2.3)
declare -A F_TEST_FILE=(
    [F-16]="gap_locking_test.rs"
    [F-23]="clustered_index_test.rs"
    [F-24]="adaptive_hash_index_test.rs"
    [F-25]="change_buffer_test.rs"
    [F-26]="double_write_buffer_test.rs"
    [F-27]="table_compression_test.rs"
    [F-29]="row_level_security_test.rs"
    [F-31]="performance_schema_test.rs"
    [F-32]="mysqladmin_test.rs"
    [F-35]="password_rotation_test.rs"
)
ISOLATED_TESTS=()
for f_id in "${ISOLATED_F_LIST[@]}"; do
    test_basename="${F_TEST_FILE[$f_id]}"
    # Find candidate test file (handle both .rs at root and in subdirs)
    test_file=$(find tests -name "$test_basename" -not -path "*/target/*" 2>/dev/null | head -1)
    if [ -z "$test_file" ]; then
        continue
    fi
    # (a) production-crate imports
    crate_uses=$(grep -c "use sqlrustgo_" "$test_file" 2>/dev/null | head -1)
    if [ -z "$crate_uses" ] || [ "$crate_uses" -eq 0 ]; then
        ISOLATED_TESTS+=("$f_id:$test_file")
    fi
done

if [ ${#ISOLATED_TESTS[@]} -gt 0 ]; then
    echo "  ⚠️  ${#ISOLATED_TESTS[@]} isolated F-XX tests detected (no 'use sqlrustgo_' import — likely self-contained mock):"
    for t in "${ISOLATED_TESTS[@]}"; do
        echo "      - ${t}"
    done
    echo "      → Per #3102, each should be promoted to a real src/ module in v3.9.0+."
    REALITY_WARN=$((REALITY_WARN + ${#ISOLATED_TESTS[@]}))
else
    echo "  ✅ 0 isolated F-XX tests (all 10 reference production crates)"
fi

# --- 5b: Detect 5 unimplemented debt items (#3103) ---
# These items have documentation-only or parser-only implementations.
# We probe for the *minimum* production-grade symbol each requires.
echo ""
echo "  --- 5 unimplemented debt items (F-03 / F-30 / F-36 / T-19 / T-20) ---"
MISSING=()
# F-03 GIS — needs geometry types
if [ -z "$(rg -l '\b(struct|enum)\s+(Point|LineString|Polygon)\b' crates/ --type rust 2>/dev/null | head -1)" ]; then
    MISSING+=("F-03 GIS: no Point/LineString/Polygon struct/enum in crates/")
fi
# F-30 SEQUENCE — needs create_sequence/nextval
if [ -z "$(rg -l 'fn\s+(create_sequence|nextval)\b' crates/ src/ --type rust 2>/dev/null | head -1)" ]; then
    MISSING+=("F-30 SEQUENCE: no create_sequence/nextval function in crates/ or src/")
fi
# F-36 column privileges — needs ColumnLevel/ColumnPrivilege
if [ -z "$(rg -l '\b(ColumnLevel|ColumnPrivilege)\b' crates/ src/ --type rust 2>/dev/null | head -1)" ]; then
    MISSING+=("F-36 列级权限: no ColumnLevel/ColumnPrivilege symbol in crates/ or src/")
fi
# T-19 Disk I/O delay — needs disk_io_delay or FAULT_INJECT_DISK
if [ -z "$(rg -l 'disk_io_delay|FAULT_INJECT_DISK' crates/ src/ tests/ --type rust 2>/dev/null | head -1)" ]; then
    MISSING+=("T-19 Disk I/O delay: no disk_io_delay/FAULT_INJECT_DISK symbol anywhere")
fi
# T-20 process_kill -9 — needs ProcessKill/process_kill or a kill-injection test
if [ -z "$(rg -l 'ProcessKill|process_kill' crates/ src/ tests/ --type rust 2>/dev/null | head -1)" ]; then
    MISSING+=("T-20 process_kill -9: no ProcessKill/process_kill symbol or test file")
fi

if [ ${#MISSING[@]} -gt 0 ]; then
    echo "  ❌ ${#MISSING[@]} unimplemented debt items confirmed missing in codebase:"
    for m in "${MISSING[@]}"; do
        echo "      - ${m}"
    done
    echo "      → Per #3103, all 5 are v3.9.0+ plan."
    REALITY_FAIL=$((REALITY_FAIL + ${#MISSING[@]}))
else
    echo "  ✅ 0 unimplemented debt items (all 5 symbols found)"
fi

# --- 5c: Detect F-XX main-path functions in execution_engine.rs (#3136) ---
# A closed F-XX must have at least one `fn execute_{keyword}_*` (or
# `fn {keyword}_*`) in src/execution_engine.rs. We map each F-id to a
# keyword derived from the test basename. Without this, the gate would
# accept any ✅ in markdown as proof of closure (文档验证文档).
echo ""
echo "  --- 10 F-XX main-path detection (#3136 Part 2: rg 'fn execute_.* {FXX}') ---"
MAIN_PATH_MISSING=()
declare -A F_KEYWORD=(
    [F-16]="gap_lock"
    [F-23]="clustered_index"
    [F-24]="adaptive_hash"
    [F-25]="change_buffer"
    [F-26]="double_write"
    [F-27]="table_compression"
    [F-29]="row_level_security"
    [F-31]="performance_schema"
    [F-32]="mysqladmin"
    [F-35]="password_rotation"
)
for f_id in "${!F_KEYWORD[@]}"; do
    kw="${F_KEYWORD[$f_id]}"
    if [ -z "$(rg -l "fn\\s+execute_${kw}\\b|fn\\s+${kw}_\\w+\\s*\\(" src/execution_engine.rs 2>/dev/null | head -1)" ] \
       && [ -z "$(rg -l "fn\\s+${kw}\\b" src/execution_engine.rs 2>/dev/null | head -1)" ]; then
        MAIN_PATH_MISSING+=("$f_id:no execute_${kw}* or ${kw}* in src/execution_engine.rs")
    fi
done
if [ ${#MAIN_PATH_MISSING[@]} -gt 0 ]; then
    echo "  ⚠️  ${#MAIN_PATH_MISSING[@]} F-XX missing main-path function in src/execution_engine.rs:"
    for m in "${MAIN_PATH_MISSING[@]}"; do
        echo "      - ${m}"
    done
    echo "      → Per #3136, these cannot be considered CLOSED without executor integration."
    REALITY_WARN=$((REALITY_WARN + ${#MAIN_PATH_MISSING[@]}))
else
    echo "  ✅ All 10 F-XX have main-path functions in src/execution_engine.rs"
fi

# --- 5d: Detect F-XX SPEC docs existence (#3136 Part 3) ---
# A closed F-XX must have a spec at docs/releases/v3.8.0/specs/debt/
# naming convention F{id-without-dash}_{KEYWORD}_SPEC.md.
echo ""
echo "  --- 10 F-XX SPEC docs detection (#3136 Part 3: docs/.../specs/debt/F{XX}_*.md) ---"
SPEC_MISSING=()
SPEC_DIR="$REPO_DIR/docs/releases/v3.8.0/specs/debt"
for f_id in "${!F_TEST_FILE[@]}"; do
    # Convert F-16 -> F16
    spec_prefix=$(echo "$f_id" | tr -d '-')
    if [ ! -d "$SPEC_DIR" ]; then
        SPEC_MISSING+=("$f_id:SPEC directory missing: $SPEC_DIR")
        continue
    fi
    if [ -z "$(ls "$SPEC_DIR"/${spec_prefix}_*_SPEC.md 2>/dev/null | head -1)" ]; then
        SPEC_MISSING+=("$f_id:no ${spec_prefix}_*_SPEC.md in specs/debt/")
    fi
done
if [ ${#SPEC_MISSING[@]} -gt 0 ]; then
    echo "  ⚠️  ${#SPEC_MISSING[@]} F-XX missing SPEC docs:"
    for m in "${SPEC_MISSING[@]}"; do
        echo "      - ${m}"
    done
    echo "      → Per #3136, CLOSED status requires spec docs."
    REALITY_WARN=$((REALITY_WARN + ${#SPEC_MISSING[@]}))
else
    echo "  ✅ All 10 F-XX have SPEC docs in specs/debt/"
fi

# --- 5e: Detect F-XX test names registered in CI D6_INTEGRATION_TESTS (#3136 Part 4) ---
# A closed F-XX must have its test file basename registered in
# scripts/gate/check_rc_ga_gate.sh D6_INTEGRATION_TESTS array. Without
# CI registration, "closed in markdown" gives no signal.
echo ""
echo "  --- 10 F-XX CI D6_INTEGRATION_TESTS registration (#3136 Part 4) ---"
CI_MISSING=()
D6_ARRAY_FILE="$REPO_DIR/scripts/gate/check_rc_ga_gate.sh"
for f_id in "${!F_TEST_FILE[@]}"; do
    test_basename="${F_TEST_FILE[$f_id]}"
    # Strip .rs extension (D6 array uses basenames without .rs)
    test_name="${test_basename%.rs}"
    if [ -f "$D6_ARRAY_FILE" ] && ! grep -qE "\"$test_name\"|^$test_name\\b" "$D6_ARRAY_FILE" 2>/dev/null; then
        CI_MISSING+=("$f_id:'$test_name' not in check_rc_ga_gate.sh D6_INTEGRATION_TESTS")
    fi
done
if [ ${#CI_MISSING[@]} -gt 0 ]; then
    echo "  ⚠️  ${#CI_MISSING[@]} F-XX not registered in CI D6_INTEGRATION_TESTS:"
    for m in "${CI_MISSING[@]}"; do
        echo "      - ${m}"
    done
    echo "      → Per #3136, CLOSED status requires CI registration."
    REALITY_WARN=$((REALITY_WARN + ${#CI_MISSING[@]}))
else
    echo "  ✅ All 10 F-XX registered in CI D6_INTEGRATION_TESTS"
fi

echo ""
# Code Reality severity: FAIL when unimplemented symbols (5b) OR
# missing F-XX main-path functions (5c) detected; otherwise WARN.
# Per #3136, missing main-path = cannot be considered CLOSED.
if [ $REALITY_FAIL -gt 0 ] || [ ${#MAIN_PATH_MISSING[@]} -gt 0 ]; then
    echo "  Code Reality: ❌ FAIL (${REALITY_FAIL} unimplemented, ${#MAIN_PATH_MISSING[@]} missing main-path, ${REALITY_WARN} isolated)"
elif [ $REALITY_WARN -gt 0 ]; then
    echo "  Code Reality: ⚠️  WARN ($REALITY_WARN isolated tests, 0 unimplemented)"
else
    echo "  Code Reality: ✅ PASS"
fi

# ============================================================
# Summary
# ============================================================
echo ""
echo "=== Cross-Version Debt Summary ==="

ACTIVE_COUNT=0
CLOSED_COUNT=0
DEFERRED_COUNT=0
PARTIAL_COUNT=0
OPEN_COUNT=0
UNKNOWN_COUNT=0

for id in "${!DEBT_STATUS[@]}"; do
    status=${DEBT_STATUS[$id]}
    case "$status" in
        ACTIVE)   ACTIVE_COUNT=$((ACTIVE_COUNT + 1)) ;;
        CLOSED)   CLOSED_COUNT=$((CLOSED_COUNT + 1)) ;;
        DEFERRED) DEFERRED_COUNT=$((DEFERRED_COUNT + 1)) ;;
        PARTIAL)  PARTIAL_COUNT=$((PARTIAL_COUNT + 1)) ;;
        OPEN)     OPEN_COUNT=$((OPEN_COUNT + 1)) ;;
        UNKNOWN)  UNKNOWN_COUNT=$((UNKNOWN_COUNT + 1)) ;;
    esac
done

echo "  Total debt items tracked: $TOTAL"
echo "  ✅ CLOSED:    $CLOSED_COUNT"
echo "  ⚠️  PARTIAL:   $PARTIAL_COUNT"
echo "  ❌ OPEN:      $OPEN_COUNT"
echo "  ⏸️  DEFERRED:  $DEFERRED_COUNT"
echo "  🔄 ACTIVE:    $ACTIVE_COUNT"
echo "  ❓ UNKNOWN:   $UNKNOWN_COUNT"

# ============================================================
# Gate criteria
# ============================================================
echo ""
echo "=== Gate Criteria Check ==="

# CRITICAL: UNKNOWN items are FAILS
if [ "$UNKNOWN_COUNT" -gt 0 ]; then
    echo "❌ FAIL: $UNKNOWN_COUNT debt items have UNKNOWN status"
    echo "    These need to be tracked in INT5_PLUS_DEBT_INVENTORY.md"
    exit 1
fi

# In strict mode, ACTIVE/OPEN fail
if [ "$STRICT_MODE" = true ]; then
    if [ "$ACTIVE_COUNT" -gt 0 ] || [ "$OPEN_COUNT" -gt 0 ]; then
        echo "❌ STRICT FAIL: $ACTIVE_COUNT ACTIVE + $OPEN_COUNT OPEN items found"
        echo "    --strict mode requires 100% CLOSED or DEFERRED"
        exit 1
    fi
fi

# Default mode: WARN on ACTIVE/OPEN, FAIL on UNKNOWN
if [ "$ACTIVE_COUNT" -gt 0 ]; then
    echo "⚠️  WARNING: $ACTIVE_COUNT ACTIVE debt items found"
    echo "    These should be moved to CLOSED or DEFERRED"
fi

if [ "$OPEN_COUNT" -gt 0 ]; then
    echo "⚠️  WARNING: $OPEN_COUNT OPEN debt items found"
    echo "    See INT5_PLUS_DEBT_INVENTORY.md for fix plans"
fi

# ADR-010 check
ADR_010="$REPO_DIR/docs/governance/adr/ADR-010-ghost-pr-resolution.md"
if [ -f "$ADR_010" ]; then
    echo "✅ ADR-010 (Ghost PR Resolution) exists - deferred items documented"
else
    echo "⚠️  WARNING: ADR-010 not found - ghost PR deferrals not documented"
fi

echo ""
echo "=== Cross-Version Debt Gate Check Complete ==="

if [ "$UNKNOWN_COUNT" -gt 0 ]; then
    echo "❌ FAIL (UNKNOWN items)"
    exit 1
elif [ "$STRICT_MODE" = true ] && { [ "$ACTIVE_COUNT" -gt 0 ] || [ "$OPEN_COUNT" -gt 0 ]; }; then
    echo "❌ STRICT FAIL"
    exit 1
else
    echo "✅ PASS"
    exit 0
fi
