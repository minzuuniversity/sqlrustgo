#!/usr/bin/env bash
#
# check_bustubx_edu_cli_smoke_v312.sh -- V312-57 BustubX-EDU sqlite-style
# CLI smoke gate (golden-file-free complement to develop's
# scripts/gate/check_bustubx_edu_cli_v312.sh).
#
# Validates that `sqlrustgo sqlite --batch <db> ...` provides a usable
# replacement for `sqlite3 edu.db < script.sql` across the week01-week04
# BustubX-EDU curriculum.
#
# Usage:  bash scripts/gate/check_bustubx_edu_cli_smoke_v312.sh
#
# Exit codes:
#   0  - all week01-week04 smoke fixtures PASS
#   1  - any fixture FAIL or build failure
#   2  - manifest missing

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

MANIFEST="$REPO_ROOT/tests/compat/bustubx_edu_sqlite_cli_smoke/manifest.yml"
FIXTURE_ROOT="$REPO_ROOT/tests/compat/bustubx_edu_sqlite_cli_smoke"

TS="$(date -u +%Y%m%dT%H%M%SZ)"
ARTIFACT_DIR="$REPO_ROOT/docs/releases/v3.12.0/evidence/v312-57-smoke/$TS"
mkdir -p "$ARTIFACT_DIR"

GATE_LOG="$ARTIFACT_DIR/gate.log"
: > "$GATE_LOG"

log() {
    printf '%s\n' "$*" | tee -a "$GATE_LOG"
}

log "=== V312-57 BustubX-EDU sqlite-style CLI Smoke Gate ==="
log "Branch:  $(git -C "$REPO_ROOT" rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
log "Commit:  $(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
log "Time:    $TS"
log "Manifest: $MANIFEST"
log ""

if [ ! -f "$MANIFEST" ]; then
    log "[FATAL] manifest not found: $MANIFEST"
    log "STATUS: BUSTUBX_EDU_CLI_SMOKE_V312_BLOCKED"
    exit 2
fi

# 1. Build
log "[1/3] cargo build -p sqlrustgo-cli --all-features"
if ! cargo build -p sqlrustgo-cli --all-features 2>>"$GATE_LOG" >>"$GATE_LOG"; then
    log "[FAIL] cargo build failed (see gate.log)"
    log "STATUS: BUSTUBX_EDU_CLI_SMOKE_V312_BLOCKED"
    exit 1
fi
log "[PASS] cargo build"

# Determine binary path. The root crate name `sqlrustgo` is the binary
# target that re-exports sqlrustgo_cli::run (per develop's
# scripts/gate/check_bustubx_edu_cli_v312.sh).
BIN=""
for candidate in \
    "$REPO_ROOT/target/debug/sqlrustgo" \
    "$REPO_ROOT/target/debug/sqlrustgo-cli" \
    ; do
    if [ -x "$candidate" ]; then
        BIN="$candidate"
        break
    fi
done

if [ -z "$BIN" ]; then
    log "[FATAL] sqlrustgo binary not found under target/debug/"
    log "STATUS: BUSTUBX_EDU_CLI_SMOKE_V312_BLOCKED"
    exit 1
fi

log "Binary: $BIN"
export SQLRUSTGO_BIN="$BIN"

# 2. Iterate fixtures
log ""
log "[2/3] running smoke fixtures (week01-week06)"

PASS=0
FAIL=0
SKIP=0
FAILED_FIXTURES=""

for week in 01 02 03 04 05 06; do
    week_dir="$FIXTURE_ROOT/week$week"
    if [ ! -d "$week_dir" ]; then
        log "  [WARN] missing week dir: $week_dir"
        continue
    fi
    for fixture in "$week_dir"/*.sh; do
        [ -e "$fixture" ] || continue
        rel="${fixture#$REPO_ROOT/}"
        FIXTURE_OUT="$("$fixture" 2>&1)"
        FIXTURE_EXIT=$?
        if [ "$FIXTURE_EXIT" -eq 0 ]; then
            if echo "$FIXTURE_OUT" | grep -q '^skip:'; then
                log "  [SKIP] $rel"
                SKIP=$((SKIP + 1))
            else
                log "  [PASS] $rel"
                PASS=$((PASS + 1))
            fi
        else
            log "  [FAIL] $rel (exit=$FIXTURE_EXIT)"
            log "         ${FIXTURE_OUT}"
            FAIL=$((FAIL + 1))
            FAILED_FIXTURES="$FAILED_FIXTURES $rel"
        fi
    done
done

# 3. Summary
log ""
log "[3/3] summary"
log "Pass:    $PASS"
log "Fail:    $FAIL"
log "Skip:    $SKIP"
log "Failed:  $FAILED_FIXTURES"
log "Artifact: $ARTIFACT_DIR/gate.log"

if [ "$FAIL" -eq 0 ]; then
    log ""
    log "STATUS: BUSTUBX_EDU_CLI_SMOKE_V312_PASS"
    exit 0
else
    log ""
    log "STATUS: BUSTUBX_EDU_CLI_SMOKE_V312_FAIL"
    exit 1
fi