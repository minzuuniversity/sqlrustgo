#!/usr/bin/env bash
# STATUS: DEPRECATED — see scripts/gate/README.md
# Reason: not invoked by any active gate (audit 2026-06-04)
# Action:  do not add new callers; restore via git history if needed

# Gate Execution Log Archive Script
# 保存每次 Gate 执行的日志，用于追溯

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$REPO_DIR"

# 参数解析
GATE_TYPE="${1:-alpha}"
COMMIT="${2:-$(git rev-parse HEAD)}"
VERSION="${3:-$(git rev-parse --abbrev-ref HEAD | grep -o 'v[0-9]\+\.[0-9]\+\.[0-9]\+' | head -1 || echo 'v3.6.0')}"

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_DIR="docs/releases/${VERSION}/logs"
mkdir -p "$LOG_DIR"

LOG_FILE="${LOG_DIR}/gate_${GATE_TYPE}_${COMMIT:0:8}_${TIMESTAMP}.log"

log() {
    local msg="$1"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $msg"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $msg" >> "$LOG_FILE"
}

exec_cmd() {
    local name="$1"
    local cmd="$2"
    local log_file="${3:-$LOG_FILE}"
    
    log "=== $name ===" 
    log "Command: $cmd"
    
    local start_time=$(date +%s)
    local output
    local exit_code=0
    
    output=$(eval "$cmd" 2>&1) || exit_code=$?
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    {
        echo "$output"
    } >> "$log_file"
    
    log "Duration: ${duration}s, Exit code: $exit_code"
    log ""
    
    return $exit_code
}

log "=== Gate Execution Log ==="
log "Gate Type: $GATE_TYPE"
log "Commit: $COMMIT"
log "Version: $VERSION"
log "Timestamp: $TIMESTAMP"
log "Log File: $LOG_FILE"
log ""

# Alpha Gate 检查
if [ "$GATE_TYPE" = "alpha" ]; then
    exec_cmd "A1: Build (release)" "cargo build --release --workspace" "$LOG_DIR/build_${TIMESTAMP}.log"
    exec_cmd "A2: Test (lib)" "cargo test --lib --workspace -- --test-threads=4" "$LOG_DIR/test_lib_${TIMESTAMP}.log"
    exec_cmd "A3: Clippy" "cargo clippy --all-features -- -D warnings" "$LOG_DIR/clippy_${TIMESTAMP}.log"
    exec_cmd "A4: Format" "cargo fmt --all -- --check" "$LOG_DIR/fmt_${TIMESTAMP}.log"
fi

# Beta Gate 检查
if [ "$GATE_TYPE" = "beta" ]; then
    exec_cmd "B1: Build (release)" "cargo build --release --workspace" "$LOG_DIR/build_${TIMESTAMP}.log"
    exec_cmd "B2: Workspace test" "cargo test --workspace -- --test-threads=4" "$LOG_DIR/test_workspace_${TIMESTAMP}.log"
    exec_cmd "B3: Clippy" "cargo clippy --all-features -- -D warnings" "$LOG_DIR/clippy_${TIMESTAMP}.log"
    exec_cmd "B4: Format" "cargo fmt --all -- --check" "$LOG_DIR/fmt_${TIMESTAMP}.log"
fi

# RC Gate 检查
if [ "$GATE_TYPE" = "rc" ]; then
    exec_cmd "R1: Full build" "cargo build --release --workspace" "$LOG_DIR/build_${TIMESTAMP}.log"
    exec_cmd "R2: Full test" "cargo test --workspace -- --test-threads=4" "$LOG_DIR/test_${TIMESTAMP}.log"
fi

# GA Gate 检查
if [ "$GATE_TYPE" = "ga" ]; then
    exec_cmd "G1: Final build" "cargo build --release --workspace" "$LOG_DIR/build_${TIMESTAMP}.log"
    exec_cmd "G2: Final test" "cargo test --workspace" "$LOG_DIR/test_${TIMESTAMP}.log"
    exec_cmd "G3: Coverage" "cargo tarpaulin --ignore-tests --out Json" "$LOG_DIR/coverage_${TIMESTAMP}.log"
fi

log "=== Gate Execution Complete ==="
log "Log file: $LOG_FILE"
log ""
log "Next steps:"
log "1. Review log file: $LOG_FILE"
log "2. Fix any failures"
log "3. Re-run gate verification"
log "4. Archive log to docs/releases/${VERSION}/logs/"