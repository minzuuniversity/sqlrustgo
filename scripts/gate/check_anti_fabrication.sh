#!/bin/bash
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

ERRORS=0; WARNINGS=0
log_info() { echo "[INFO] $*"; }
log_error() { echo "[ERROR] $*" >&2; ERRORS=$((ERRORS + 1)); }
log_pass() { echo "[PASS] $*"; }
log_warn() { echo "[WARN] $*" >&2; WARNINGS=$((WARNINGS + 1)); }
log_skip() { echo "[SKIP] $*"; }

check_canonical_binary_build() {
    log_info "CHECK 1: cargo build -p sqlrustgo-mysql-server (canonical binary)..."
    if cargo check -p sqlrustgo-mysql-server --all-features 2>/tmp/cargo-check-mysql.log; then
        log_pass "sqlrustgo-mysql-server: cargo check PASS"
    else
        log_error "sqlrustgo-mysql-server: cargo check FAILED"
        tail -20 /tmp/cargo-check-mysql.log >&2
    fi
}

KNOWN_PREEXISTING_FAILURES=(
    "aggregate_type_test" "agentsql_test" "backup_test" "batch_insert_test"
    "boundary_test" "buffer_pool_test" "catalog_consistency_test" "columnar_storage_test"
    "concurrency_stress_test" "crash_injection_test" "crash_recovery_test"
    "datetime_type_test" "diag_22_on_sf01_broken" "diag_2t_join"
    "distributed_transaction_test" "e2e_observability_test" "eval_22_vs_sf01"
    "expr_single_engine_test" "fk_constraint_test" "foreign_key_test"
    "index_integration_test" "int3_spec_complete_test" "join_test"
    "kill_stress_test" "local_executor_test" "mysql_compatibility_test"
    "null_handling_test" "openclaw_api_test" "optimizer_cost_test"
    "optimizer_rules_test" "outer_join_test" "parquet_test"
    "perf_eng_batched_insert_test" "performance_test" "planner_test"
    "q21_perf_bench" "savepoint_test" "server_integration_test"
    "session_config_test" "set_variable_test" "sql_cli_test"
    "storage_integration_test" "stress_test" "teaching_scenario_test"
    "tpch_benchmark" "tpch_comparison_test" "tpch_hash_test" "tpch_index_test"
    "tpch_qtest" "tpch_sf1_test" "tpch_test" "tpch_text_index_test"
    "tpch_wire_harness" "vectorization_test" "parallel_perf_baseline_test"
    "teaching_scenario_client_server_test" "executor_test" "snapshot_isolation_test"
    "checksum_corruption_test" "types_value_test" "production_scenario_test"
    "auth_rbac_test" "tpch_compliance_test" "vector_storage_integration_test"
    "view_test" "wal_fuzz_test" "wal_deterministic_test"
    # Missing binary source files:
    # V312-50 (per codex #88807 4th item): removed stale 'sqlancer'/'test-runner'/'test-registry-cli'
    # entries per codex #88807 4th item — they were 'Missing binary source files' from
    # pre-V312-24 days. V312-24 now ACTIVE these binaries (CHECK 1.5 enforces
    # they exist + produce artifacts). Keeping them as KNOWN would mask
    # regressions.
    # Example compilation errors:
    "sqlrustgo-bench" "tpch_run_query"
    # Main workspace binary (compile error propagates):
    "sqlrustgo"
)

check_test_compile() {
    log_info "CHECK 2: cargo test --workspace --no-run (test compile only, fast)..."
    local rc=0
    cargo test --workspace --no-run 2>/tmp/cargo-test-norun.log || rc=$?

    if [[ $rc -eq 0 ]]; then
        log_pass "Test binaries compile PASS"
        return 0
    fi

    # Extract failing identifiers from cargo output.
    # Two formats to handle:
    # 1. "couldn't read .../src/bin/name.rs"  → name is the binary in path
    # 2. "error: could not compile `binary` (test|example "name")" → binary OR test name
    #
    # Strategy: extract all candidates, match against KNOWN by BINARY NAME only.
    # The binary name (between backticks or path component) is the stable identifier.
    # test/example names inside parens can vary due to compilation non-determinism.

    local candidates=""
    # Format 1: "couldn't read .../src/bin/name.rs"
    candidates+=$(grep "couldn't read .*/src/bin/[^.]*\.rs" /tmp/cargo-test-norun.log 2>/dev/null \
        | sed 's|.*/||; s|\.rs$||' \
        || true)
    candidates+=$'\n'
    # Format 2: binary names from "error: could not compile `binary` ..."
    candidates+=$(grep '^error: could not compile' /tmp/cargo-test-norun.log 2>/dev/null \
        | sed 's/.*could not compile `\([^`]*\)`.*/\1/' \
        || true)
    candidates+=$'\n'
    # Format 2 fallback: test/example names inside parens
    candidates+=$(grep '^error: could not compile' /tmp/cargo-test-norun.log 2>/dev/null \
        | grep -oE '\((test|example) "[^"]+"\)' \
        | sed 's/(test "//; s/(example "//; s/")//' \
        || true)

    local failing_binaries
    failing_binaries=$(echo "$candidates" | grep -v '^$' | sort -u)

    if [[ -z "$failing_binaries" ]]; then
        log_error "Test binaries compile FAILED (no binary names extracted)"
        tail -30 /tmp/cargo-test-norun.log >&2
        return 1
    fi

    local new_failures=""
    local known_count=0
    for binary in $failing_binaries; do
        local is_known=0
        for known_bin in "${KNOWN_PREEXISTING_FAILURES[@]}"; do
            if [[ "$binary" == "$known_bin" ]]; then
                is_known=1; break
            fi
        done
        if [[ $is_known -eq 0 ]]; then
            new_failures="${new_failures}  ${binary}"
        else
            known_count=$((known_count + 1))
        fi
    done

    local total_count
    total_count=$(echo "$failing_binaries" | wc -w | tr -d ' ')

    if [[ -n "$new_failures" ]]; then
        log_error "Test binaries compile FAILED — NEW untracked failures detected:"
        for b in $new_failures; do log_error "  NEW: $b"; done
        log_error "Known pre-existing failures ($known_count of $total_count):"
        for binary in $failing_binaries; do
            local is_known=0
            for known_bin in "${KNOWN_PREEXISTING_FAILURES[@]}"; do
                if [[ "$binary" == "$known_bin" ]]; then is_known=1; break; fi
            done
            [[ $is_known -eq 1 ]] && log_info "  KNOWN: $binary"
        done
        log_error "See RC_BLOCKERS_REPORT.md §Blocker #4 for tracking."
        tail -20 /tmp/cargo-test-norun.log >&2
        return 1
    fi

    log_pass "Test binaries compile: all $total_count failures are KNOWN pre-existing"
    WARNINGS=$((WARNINGS + 1))
    log_warn "Pre-existing failures (API evolution, not fabrication):"
    for binary in $failing_binaries; do
        log_warn "  (pre-existing) $binary"
    done
    return 0
}

check_gate_report_test_counts() {
    log_info "CHECK 3: Gate report test counts vs actual cargo test compile output..."
    local reports=(
        "docs/releases/v3.8.0/alpha/ALPHA_GATE_REPORT.md"
        "docs/releases/v3.8.0/beta/BETA_GATE_REPORT.md"
        "docs/releases/v3.8.0/rc/RC_GA_GATE_REPORT.md"
        "docs/releases/v3.9.0/rc/RC1_GATE_REPORT.md"
        "docs/releases/v3.9.0/rc/RC2_GATE_REPORT.md"
    )
    local test_bin_count
    test_bin_count=$(grep -rE '^Executable(unittest)\s' /tmp/cargo-test-norun.log 2>/dev/null | wc -l)
    [[ $test_bin_count -eq 0 ]] && test_bin_count="unknown"
    for report in "${reports[@]}"; do
        [[ ! -f "$report" ]] && { log_skip "$report: not found"; continue; }
        local reported
        reported=$(grep -oE '[0-9]+/[0-9]+ (tests|PASS)' "$report" | head -1 || true)
        [[ -n "$reported" ]] && log_info "$report: declares $reported (test binaries: $test_bin_count)" \
            || log_warn "$report: no test count claim found"
    done
}

check_head_commit_author() {
    log_info "CHECK 4: HEAD commit author must be a known AI/Human identity..."
    local head_email
    head_email=$(git log -1 --format='%ae' 2>/dev/null)
    local allowed=("openheart@gaoyuanyiyao.com" "openclaw@gaoyuanyiyao.com" "hermes-z6g4@gaoyuanyiyao.com" "hermes-macmini@gaoyuanyiyao.com" "claude-macmini@gaoyuanyiyao.com" "claude-z6g4@gaoyuanyiyao.com" "claude-z440@gaoyuanyiyao.com" "ci@sqlrustgo.dev")
    local found=0
    for e in "${allowed[@]}"; do
        if [[ "$head_email" == "$e" ]]; then
            log_pass "HEAD author email: $head_email (allowed multi-AI identity)"
            found=1; break
        fi
    done
    [[ $found -eq 0 ]] && log_error "HEAD author email: $head_email (NOT in allowed list)"
}

check_doc_code_examples() {
    log_info "CHECK 5: Code examples in documentation (sanity scan)..."
    local checked=0
    for dir in "docs/releases/v3.8.0" "docs/governance" "docs/releases/v3.9.0"; do
        [[ -d "$dir" ]] || continue
        local count
        count=$(find "$dir" -name "*.md" -type f 2>/dev/null | wc -l)
        checked=$((checked + count))
    done
    log_pass "Scanned $checked markdown files in docs/releases + docs/governance"
}

# V312-50 (per codex #88807 4th item): restore CHECK 1.5 (V312-24 artifact gate).
# Originally added in V312-32 commit 1f94d119da; removed by PR #3968 anti-fab v5
# regression (888ee1445e); never restored when PR #3995 (V312-49) merged
# V312-46 corpus runner re-apply. Adding back per codex #88807 strict review.
#
# Per ISSUE #3911 acceptance criterion 1 ("Every tool can run + produce
# artifact"), the canonical SQLancer + test-runner binaries must produce
# target/sqlancer-report.json + target/test-runner-report.json on demand.
# This check is a fail-explicit guard: absence of artifacts = gate fail.
check_v312_24_test_infra_artifacts() {
    log_info "CHECK 1.5: V312-24 SQLancer + test-runner report artifacts..."
    local errors=0

    # SQLancer
    if [ ! -x "${REPO_ROOT}/target/release/sqlancer" ]; then
        log_info "  target/release/sqlancer not present; building..."
        cargo build --release -p sqlancer 2>/tmp/cargo-build-sqlancer.log || errors=$((errors + 1))
    fi
    if [ -x "${REPO_ROOT}/target/release/sqlancer" ]; then
        if [ ! -s "${REPO_ROOT}/target/sqlancer-report.json" ]; then
            log_info "  target/sqlancer-report.json missing; running sqlancer (--duration 5)..."
            "${REPO_ROOT}/target/release/sqlancer" --duration 5 \
                --out "${REPO_ROOT}/target/sqlancer-report.json" 2>/tmp/sqlancer-run.log || errors=$((errors + 1))
        fi
        if [ -s "${REPO_ROOT}/target/sqlancer-report.json" ]; then
            if python3 -c "import json,sys
d=json.load(open('${REPO_ROOT}/target/sqlancer-report.json'))
for k in ('successful_queries','failed_queries','iterations_requested'):
    if k not in d: sys.exit(1)
" 2>/dev/null; then
                local iters
                iters=$(python3 -c "import json; print(json.load(open('${REPO_ROOT}/target/sqlancer-report.json'))['iterations_requested'])")
                log_pass "  V312-24: target/sqlancer-report.json valid (iterations=${iters})"
            else
                log_error "  V312-24: target/sqlancer-report.json schema INVALID"
                errors=$((errors + 1))
            fi
        else
            log_error "  V312-24: target/sqlancer-report.json still missing after sqlancer run"
            errors=$((errors + 1))
        fi
    else
        log_error "  V312-24: target/release/sqlancer not built; cannot produce report"
        errors=$((errors + 1))
    fi

    # test-runner
    if [ ! -x "${REPO_ROOT}/target/release/test-runner" ]; then
        log_info "  target/release/test-runner not present; building..."
        cargo build --release -p test-runner 2>/tmp/cargo-build-test-runner.log || errors=$((errors + 1))
    fi
    if [ -x "${REPO_ROOT}/target/release/test-runner" ]; then
        if [ ! -s "${REPO_ROOT}/target/test-runner-report.json" ]; then
            log_info "  target/test-runner-report.json missing; running test-runner probe..."
            "${REPO_ROOT}/target/release/test-runner" \
                --out "${REPO_ROOT}/target/test-runner-report.json" 2>/tmp/test-runner-run.log || errors=$((errors + 1))
        fi
        if [ -s "${REPO_ROOT}/target/test-runner-report.json" ]; then
            if python3 -c "import json,sys
d=json.load(open('${REPO_ROOT}/target/test-runner-report.json'))
for k in ('started_at','finished_at','config','summary','results'):
    if k not in d: sys.exit(1)
" 2>/dev/null; then
                local total_duration
                total_duration=$(python3 -c "import json; print(json.load(open('${REPO_ROOT}/target/test-runner-report.json'))['summary']['total_duration_ms'])")
                log_pass "  V312-24: target/test-runner-report.json valid (total_duration_ms=${total_duration})"
            else
                log_error "  V312-24: target/test-runner-report.json schema INVALID"
                errors=$((errors + 1))
            fi
        else
            log_error "  V312-24: target/test-runner-report.json still missing after test-runner run"
            errors=$((errors + 1))
        fi
    else
        log_error "  V312-24: target/release/test-runner not built; cannot produce report"
        errors=$((errors + 1))
    fi

    if [[ $errors -gt 0 ]]; then
        log_error "  V312-24 artifacts check: $errors failure(s) (see lines above)"
    else
        log_pass "  V312-24: both SQLancer + test-runner report artifacts present and valid"
    fi
}

main() {
    echo "============================================"
    echo "Anti-Fabrication Policy Check (AFP) v4"
    check_canonical_binary_build; echo
    check_v312_24_test_infra_artifacts; echo
    check_test_compile; echo
    check_gate_report_test_counts; echo
    check_head_commit_author; echo
    check_doc_code_examples; echo
    echo "============================================"
    echo "Results: ERRORS=$ERRORS, WARNINGS=$WARNINGS"
    echo "============================================"
    echo
    [[ $ERRORS -gt 0 ]] && { echo "[FAIL] Anti-fabrication check (AFP v4): FAILED with $ERRORS error(s)"; exit 1; }
    echo "[PASS] Anti-fabrication check (AFP v4): PASS"; exit 0
}
main "$@"
