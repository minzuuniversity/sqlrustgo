#!/usr/bin/env bash
# scripts/gate/mutate_tpch_oracle_tests.sh
#
# Mutation testing for the TPC-H assertions introduced by issue #5379.
#
# # Why
#
# The 13 TPC-H diagnostic tests in `tests/integration/oracle/` used to have
# ZERO assertions — they printed the expected value with `eprintln!` and
# nothing else, so they "passed" even when every query returned garbage.
# Issue #5379 gave them real assertions (an independent in-process SQLite
# oracle plus data-independent invariants). A test that cannot fail proves
# nothing, so the new assertions have to be shown to actually *kill* defects.
#
# # What is mutated
#
# The **engine implementation**, never the test code: mutating the test
# would be self-fulfilling. Three mutations are injected into the query
# execution path of `src/engine_select.rs`:
#
#   MUT-1  HAVING predicate inverted   — the row filter keeps the wrong rows
#   MUT-2  SUM drops the first value   — the aggregate is subtly wrong
#   MUT-3  COUNT(DISTINCT x) ignored   — the de-duplication is lost
#
# `tests/integration/oracle/diag_q11.rs` carries the SQLite comparison AND
# the data-independent invariants, so it is the target that must kill MUT-1
# (HAVING) and MUT-2 (SUM). MUT-3 (COUNT DISTINCT) needs a target where
# COUNT(DISTINCT x) actually differs from COUNT(x) — on this fixture
# `diag_q11`'s `COUNT(DISTINCT ps_partkey)` over the GERMANY join equals
# `COUNT(ps_partkey)` (320 = 320, every partkey occurs exactly once), so the
# mutation would be invisible there. `diag_q12` compares
# `COUNT(DISTINCT l_shipmode)` against the SQLite oracle, where the distinct
# count is ~7 and the plain count is ~10^4 — the mutation is unmissable.
#
# **The anchor of MUT-1 must cover BOTH HAVING paths.** The first version of
# this script anchored only on the no-GROUP-BY path
# (`if !eval_predicate_with_subq(...)`); `diag_q11` exercises the
# with-GROUP-BY path (`agg_result_rows.retain(|row| eval(...))`), so the
# mutation never reached the code under test and MUT-1 SURVIVED on CI
# (run 8115, mutation job 16571: CAUGHT=2 SURVIVED=1 ERRORS=0). The gate
# reported the survival honestly instead of faking a pass — that is what a
# mutation gate is for.
#
# # Impact analysis (CLAUDE.md requires it before touching a symbol)
#
# `gitnexus impact eval_predicate_with_subq` → risk CRITICAL, 21 impacted
# symbols, 5 execution flows (execute_insert / execute_update /
# execute_delete / ...). That is expected: the mutated predicate evaluator is
# shared. The mutation is applied for the duration of one test run and
# reverted immediately afterwards; the script aborts with the file restored.
#
# # Usage
#
#   bash scripts/gate/mutate_tpch_oracle_tests.sh
#
# The fixture directory is NEVER removed by this script (it lives in
# $RUNNER_TEMP or $TMPDIR and is left in place for post-mortem inspection);
# there is no --keep-fixture switch, because there is nothing to switch off.
#
# Exit codes: 0 = every mutation CAUGHT · 1 = a mutation SURVIVED
#             2 = the harness itself failed (mutation did not apply, build
#                 error, missing fixture, ...)

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT" || exit 2

RED=$'\033[0;31m'
GREEN=$'\033[0;32m'
YELLOW=$'\033[0;33m'
NC=$'\033[0m'

FIXTURE_DIR="${TPCH_MUTATION_FIXTURE_DIR:-${RUNNER_TEMP:-/tmp}/tpch-mutation-fixture}"
ENGINE_FILE="src/engine_select.rs"

# The target that can actually detect a given mutation. A mutation paired
# with a target that cannot express it would "survive" for the wrong reason.
# Wall-clock bound for a single baseline/mutation **test execution**. The
# diagnostic targets normally finish in well under a minute; 15 minutes is a
# generous ceiling that still keeps a hung run from eating the whole CI
# container.
RUN_TIMEOUT_S="${RUN_TIMEOUT_S:-900}"

# Wall-clock bound for the **compile** phase — a separate budget, because the
# two phases have completely different costs.
#
# The CI job container has **no cargo cache**: a fresh container per job and
# an ephemeral `$CARGO_HOME`, so every `cargo test` starts by compiling the
# workspace's dependency tree from scratch. Measured on this repository:
# `cargo test --test diag_q11` needs **5m57s** to compile, after which the
# test itself runs in **0.19s**.
#
# The old single 900s budget had to cover *both* phases, and on the CI runner
# — where several jobs compete for the same CPU — the compile alone exceeded
# it: mutation job 18184 died with `rc=143` after exactly 900s, i.e. the
# timeout fired while cargo was still compiling. The compile is therefore
# done **once**, up front, with its own budget; the run budget then only has
# to cover the test execution.
COMPILE_TIMEOUT_S="${COMPILE_TIMEOUT_S:-3600}"

# `timeout` is not part of a bare macOS install (coreutils' `gtimeout` is),
# but the CI runner is Linux and the repository already relies on `timeout`
# elsewhere. Fail with a clear message rather than with "command not found".
if ! command -v timeout >/dev/null 2>&1; then
    echo "ERROR: 'timeout' is required to bound the mutation runs (install coreutils or set PATH)" >&2
    exit 2
fi

MUT1_TARGET="diag_q11"
MUT2_TARGET="diag_q11"
MUT3_TARGET="diag_q12"

CAUGHT=0
SURVIVED=0
ERRORS=0

# Checksum of the pristine engine file — used to prove that every mutation
# left no residue behind.
ENGINE_SHA_BEFORE=""

log() { printf '%s\n' "$*"; }

# --- temporary files and digests ---------------------------------------------
#
# Both helpers now live in `scripts/gate/common.sh` — one implementation of
# each rule, so a gate that re-invents them cannot drift into a second set
# of portability assumptions (the bare-`mktemp` trap on the CI runner image
# is documented there).
# shellcheck source=scripts/gate/common.sh
source "$REPO_ROOT/scripts/gate/common.sh"

# --- ensure a fixture exists -------------------------------------------------
if [ ! -f "$FIXTURE_DIR/lineitem.tbl" ]; then
    log "== preparing fixture: $FIXTURE_DIR"
    if ! bash scripts/gate/prepare_tpch_fixture.sh --out "$FIXTURE_DIR" --sf 0.01 --seed 42; then
        log "${RED}ERROR${NC}: could not prepare the fixture"
        exit 2
    fi
fi

# --- restore the engine file on any exit ------------------------------------
restore_engine() {
    if [ -n "${BACKUP_FILE:-}" ] && [ -f "$BACKUP_FILE" ]; then
        cp "$BACKUP_FILE" "$ENGINE_FILE"
        rm -f "$BACKUP_FILE"
        BACKUP_FILE=""
        log "   engine file restored"
    fi
}
trap restore_engine EXIT INT TERM

# --- prove that the engine file is left byte-identical ----------------------
assert_engine_pristine() {
    if [ -z "$ENGINE_SHA_BEFORE" ]; then
        return
    fi
    local now
    now="$(sha256_of "$ENGINE_FILE")"
    if [ "$now" != "$ENGINE_SHA_BEFORE" ]; then
        log "${RED}ERROR${NC}: $ENGINE_FILE was not restored to its pristine state (sha256 $now != $ENGINE_SHA_BEFORE)"
        ERRORS=$((ERRORS + 1))
    fi
}

# --- print the tail of a run's output ---------------------------------------
#
# A failed run must be **diagnosable from the CI log alone**. The narrow grep
# that used to be the only output (`^test .* FAILED|panicked at|error\[|^error`)
# matched nothing when cargo died before it produced a test result — which is
# exactly the case that mattered: mutation job 18223 reported
# `the BASELINE run of 'diag_q11' failed (rc=101)` with **no** diagnostic line
# at all, so the cause had to be recovered by hand from the raw job log.
#
# The tail is printed **in addition to** the grep, never instead of it: the
# grep stays the first thing a reader sees, and the tail carries everything
# the grep cannot know about — cargo's download errors, compiler errors,
# linker errors, and the OOM killer's output.
report_run_output() {
    local out="$1"
    local lines="${2:-40}"
    if [ ! -f "$out" ]; then
        log "   (no output file was produced)"
        return 0
    fi
    log "   --- last ${lines} lines of the run output ---"
    tail -n "$lines" "$out" | sed 's/^/   | /'
    log "   --- end of the run output ---"
}

# --- compile the mutation targets, once -------------------------------------
#
# The compile phase and the test-execution phase have completely different
# costs (see COMPILE_TIMEOUT_S), so they get separate budgets. Compiling up
# front also means the three mutation runs reuse the build products: after a
# mutation only the mutated engine crate is recompiled, instead of rebuilding
# the whole dependency tree four times.
compile_targets() {
    local out rc
    local targets=""
    local cargo_args=""
    local t
    for t in "$MUT1_TARGET" "$MUT2_TARGET" "$MUT3_TARGET"; do
        case " $targets " in
            *" $t "*) continue ;;
        esac
        targets="$targets $t"
    done
    for t in $targets; do
        cargo_args="$cargo_args --test $t"
    done

    out="$(make_temp_file)" || {
        log "${RED}ERROR${NC}: cannot create the compile output file"
        return 1
    }
    log ""
    log "== compiling the mutation targets (budget ${COMPILE_TIMEOUT_S}s) — the CI container has no cargo cache, so this is the expensive part"
    # `cargo_args` is intentionally unquoted: it must word-split into
    # `--test <name>` pairs. Target names contain no spaces.
    # shellcheck disable=SC2086
    timeout "$COMPILE_TIMEOUT_S" cargo test --no-run $cargo_args >"$out" 2>&1
    rc=$?

    if [ $rc -eq 124 ] || [ $rc -eq 143 ]; then
        log "${RED}ERROR${NC}: compiling the mutation targets TIMED OUT after ${COMPILE_TIMEOUT_S}s"
        report_run_output "$out" 40
        rm -f "$out"
        return 1
    fi
    if [ $rc -ne 0 ]; then
        log "${RED}ERROR${NC}: the mutation targets could not be compiled (rc=$rc)"
        report_run_output "$out" 40
        rm -f "$out"
        return 1
    fi
    log "   compiled OK"
    rm -f "$out"
    return 0
}

# --- baseline run -----------------------------------------------------------
# The mutations are only meaningful if the *unmutated* test passes. Without
# this baseline a pre-existing failure (a broken fixture, a stale expected
# value) would make every mutation look "CAUGHT" — the gate would report a
# green result while proving nothing. A failing baseline is a harness error
# (exit 2), never a caught mutation.
run_baseline() {
    local target="$1"
    log ""
    log "== BASELINE: the unmutated '$target' must pass, otherwise CAUGHT is meaningless"
    local out rc
    out="$(make_temp_file)" || {
        log "${RED}ERROR${NC}: cannot create the baseline output file"
        return 1
    }
    # The run is bounded: a gate that can hang is a gate that eats the whole
    # CI container. A timeout is reported as a harness error, never as a
    # "caught" mutation — the point of a mutation run is that the *test*
    # failed, not that the process died.
    TPCH_DATA_DIR="$FIXTURE_DIR" timeout "$RUN_TIMEOUT_S" \
        cargo test --test "$target" -- --nocapture >"$out" 2>&1
    rc=$?
    # 124 is GNU timeout's "timed out"; 143 is what the shell reports when
    # the timeout signal reaches the job itself. Both mean the run did not
    # finish, and neither is evidence about the test.
    if [ $rc -eq 124 ] || [ $rc -eq 143 ]; then
        log "${RED}ERROR${NC}: the BASELINE run of '$target' TIMED OUT after ${RUN_TIMEOUT_S}s"
        report_run_output "$out" 40
        rm -f "$out"
        return 1
    fi
    if [ $rc -ne 0 ]; then
        log "${RED}ERROR${NC}: the BASELINE run of '$target' failed (rc=$rc) — the mutation results would be meaningless"
        grep -E "^test .* FAILED|panicked at|error\[|^error" "$out" | head -10
        report_run_output "$out" 60
        rm -f "$out"
        return 1
    fi
    log "   baseline OK"
    rm -f "$out"
    return 0
}

# --- apply one mutation, run the target, classify the outcome ---------------
# $1 mutation id · $2 description · $3 python replacement snippet file
#   (the snippet reads the engine file and rewrites it in place)
run_mutation() {
    local id="$1"
    local desc="$2"
    local snippet="$3"
    local target="$4"

    log ""
    log "== ${id}: ${desc}  (target: ${target})"

    BACKUP_FILE="$(make_temp_file)" || {
        log "${RED}ERROR${NC}: cannot create the engine backup file"
        ERRORS=$((ERRORS + 1))
        return
    }
    cp "$ENGINE_FILE" "$BACKUP_FILE" || {
        log "${RED}ERROR${NC}: cannot back up $ENGINE_FILE"
        ERRORS=$((ERRORS + 1))
        return
    }

    if ! python3 "$snippet" "$ENGINE_FILE"; then
        log "${RED}ERROR${NC}: ${id}: the mutation anchor was not found exactly once — the mutation was NOT applied"
        ERRORS=$((ERRORS + 1))
        restore_engine
        return
    fi

    local out rc
    out="$(make_temp_file)" || {
        log "${RED}ERROR${NC}: cannot create the mutation output file"
        ERRORS=$((ERRORS + 1))
        restore_engine
        return
    }
    TPCH_DATA_DIR="$FIXTURE_DIR" timeout "$RUN_TIMEOUT_S" \
        cargo test --test "$target" -- --nocapture >"$out" 2>&1
    rc=$?

    if [ $rc -eq 124 ] || [ $rc -eq 143 ]; then
        # A timeout is NOT a caught mutation: the test did not fail, the
        # process was killed. Reporting it as CAUGHT would make the gate
        # green for the wrong reason.
        log "${RED}ERROR${NC}: ${id}: the mutated run TIMED OUT after ${RUN_TIMEOUT_S}s — this is a harness error, not a caught mutation"
        report_run_output "$out" 40
        ERRORS=$((ERRORS + 1))
        rm -f "$out"
        restore_engine
        assert_engine_pristine
        return
    fi

    if [ $rc -ne 0 ]; then
        log "${GREEN}CAUGHT${NC} (${id}): the test failed as it should"
        grep -E "^test .* FAILED|panicked at|differs from the independent SQLite oracle" "$out" | head -5
        report_run_output "$out" 25
        CAUGHT=$((CAUGHT + 1))
    else
        log "${RED}SURVIVED${NC} (${id}): the test still passed — the assertion does not detect this defect"
        log "   ${YELLOW}先怀疑夹具/检测器，再怀疑引擎${NC}: 变异是否真的改变了行为？该用例是否真的执行到了这段代码路径？"
        SURVIVED=$((SURVIVED + 1))
    fi
    rm -f "$out"
    restore_engine
    assert_engine_pristine
}

# --- mutation snippets ------------------------------------------------------
# Three standalone temp files (no directory) so cleanup never needs `rm -rf`.
MUT1_SNIPPET="$(make_temp_file)" || { log "${RED}ERROR${NC}: cannot create the MUT-1 snippet file"; exit 2; }
MUT2_SNIPPET="$(make_temp_file)" || { log "${RED}ERROR${NC}: cannot create the MUT-2 snippet file"; exit 2; }
MUT3_SNIPPET="$(make_temp_file)" || { log "${RED}ERROR${NC}: cannot create the MUT-3 snippet file"; exit 2; }
trap 'rm -f "$MUT1_SNIPPET" "$MUT2_SNIPPET" "$MUT3_SNIPPET"; restore_engine; assert_engine_pristine' EXIT INT TERM

cat > "$MUT1_SNIPPET" <<'PY'
# MUT-1: invert the HAVING predicate on BOTH of the engine's HAVING paths.
#
# `src/engine_select.rs` implements HAVING twice:
#   (a) no GROUP BY (a whole-table aggregate): `if !eval(having)` guards the
#       "the HAVING predicate is false -> return zero rows" branch, so
#       dropping the `!` inverts that path;
#   (b) with GROUP BY: `agg_result_rows.retain(|row| eval(having))` keeps the
#       groups whose HAVING predicate is TRUE, so adding the `!` inverts that
#       path.
#
# BOTH must be inverted. A mutation that touches only one of them is invisible
# to every test that exercises the other one — which is exactly how MUT-1
# survived on CI (run 8115, mutation job 16571: CAUGHT=2 SURVIVED=1 ERRORS=0):
# the anchor sat on path (a) while `diag_q11`'s `GROUP BY … HAVING` goes
# through path (b), so the mutation never touched the code under test.
import sys

path = sys.argv[1]

REPLACEMENTS = [
    # (a) no GROUP BY — drop the `!`
    (
        """                    if !crate::engine_utils::eval_predicate_with_subq(
                        having_expr,""",
        """                    if crate::engine_utils::eval_predicate_with_subq(
                        having_expr,""",
    ),
    # (b) with GROUP BY — add the `!`
    (
        """                    agg_result_rows.retain(|row| {
                        crate::engine_utils::eval_predicate_with_subq(
                            having_expr,""",
        """                    agg_result_rows.retain(|row| {
                        !crate::engine_utils::eval_predicate_with_subq(
                            having_expr,""",
    ),
]

text = open(path, encoding="utf-8").read()
for old, new in REPLACEMENTS:
    count = text.count(old)
    if count != 1:
        print(f"anchor found {count} times (expected 1):\n{old}", file=sys.stderr)
        sys.exit(1)
    text = text.replace(old, new)
open(path, "w", encoding="utf-8").write(text)
print("MUT-1 applied: HAVING predicate inverted on BOTH implementation paths")
PY

cat > "$MUT2_SNIPPET" <<'PY'
# MUT-2: SUM silently drops the first value of each group.
import sys

path = sys.argv[1]
old = """                    let mut any_float = false;
                    for v in &values {
                        match v {"""
new = """                    let mut any_float = false;
                    for v in values.iter().skip(1) {
                        match v {"""
text = open(path, encoding="utf-8").read()
count = text.count(old)
if count != 1:
    print(f"anchor found {count} times (expected 1)", file=sys.stderr)
    sys.exit(1)
open(path, "w", encoding="utf-8").write(text.replace(old, new))
print("MUT-2 applied: SUM drops the first value")
PY

cat > "$MUT3_SNIPPET" <<'PY'
# MUT-3: COUNT(DISTINCT x) loses its de-duplication.
#
# The DISTINCT branch is replaced by a plain non-NULL count, i.e. the
# mutation makes COUNT(DISTINCT x) behave exactly like COUNT(x).
#
# NOTE: an earlier version of this mutation made the `COUNT(*)` branch
# unreachable instead (`if false { unreachable!() } else if ...`). That was a
# NO-OP: the `else if agg.distinct` branch stayed reachable, so the engine
# behaved identically and the mutation "survived" for the wrong reason. The
# current form keeps every branch reachable and changes the computed value.
import sys

path = sys.argv[1]
old = """                    } else if agg.distinct {
                        // COUNT(DISTINCT col) - count unique non-NULL values
                        use std::collections::HashSet;
                        let unique: HashSet<_> = values
                            .iter()
                            .filter(|v| !matches!(v, Value::Null))
                            .collect();
                        Value::Integer(unique.len() as i64)
                    } else {"""
new = """                    } else if agg.distinct {
                        // MUT-3: the de-duplication is lost — every non-NULL
                        // value is counted, not every distinct value.
                        Value::Integer(
                            values.iter().filter(|v| !matches!(v, Value::Null)).count() as i64,
                        )
                    } else {"""
text = open(path, encoding="utf-8").read()
count = text.count(old)
if count != 1:
    print(f"anchor found {count} times (expected 1)", file=sys.stderr)
    sys.exit(1)
open(path, "w", encoding="utf-8").write(text.replace(old, new))
print("MUT-3 applied: COUNT(DISTINCT x) de-duplication lost")
PY

# --- run --------------------------------------------------------------------
log "=== TPC-H assertion mutation testing (issue #5379) ==="
log "engine file : $ENGINE_FILE"
log "fixture     : $FIXTURE_DIR"

ENGINE_SHA_BEFORE="$(sha256_of "$ENGINE_FILE")"
if [ -z "$ENGINE_SHA_BEFORE" ]; then
    log "${RED}ERROR${NC}: cannot checksum $ENGINE_FILE"
    exit 2
fi

# One baseline per distinct target. Every target used below must pass while
# unmutated, otherwise a pre-existing failure would make each mutation look
# "CAUGHT" and the gate would report green while proving nothing.
#
# The compile happens once, before any baseline: the CI container has no cargo
# cache, so paying for the dependency tree four times (once per baseline and
# once per mutation) is what made the old single 900s budget unreachable.
if ! compile_targets; then
    exit 2
fi

BASELINES_DONE=""
for t in "$MUT1_TARGET" "$MUT2_TARGET" "$MUT3_TARGET"; do
    case " $BASELINES_DONE " in
        *" $t "*) continue ;;
    esac
    if ! run_baseline "$t"; then
        exit 2
    fi
    BASELINES_DONE="$BASELINES_DONE $t"
done

run_mutation "MUT-1" "HAVING predicate inverted" "$MUT1_SNIPPET" "$MUT1_TARGET"
run_mutation "MUT-2" "SUM drops the first value of each group" "$MUT2_SNIPPET" "$MUT2_TARGET"
run_mutation "MUT-3" "COUNT(DISTINCT x) loses de-duplication" "$MUT3_SNIPPET" "$MUT3_TARGET"

assert_engine_pristine

log ""
log "=== mutation summary ==="
log "CAUGHT   : $CAUGHT"
log "SURVIVED : $SURVIVED"
log "ERRORS   : $ERRORS"

if [ "$ERRORS" -gt 0 ]; then
    log "${RED}FAIL${NC}: the mutation harness itself failed (${ERRORS} error(s)) — the result is not trustworthy"
    exit 2
fi
if [ "$SURVIVED" -gt 0 ]; then
    log "${RED}FAIL${NC}: ${SURVIVED} mutation(s) survived — the new assertions do not detect them"
    exit 1
fi
if [ "$CAUGHT" -eq 0 ]; then
    log "${YELLOW}FAIL${NC}: no mutation was executed"
    exit 2
fi
log "${GREEN}PASS${NC}: all ${CAUGHT} mutation(s) were caught by the TPC-H assertions"
exit 0
