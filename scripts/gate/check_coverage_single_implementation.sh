#!/usr/bin/env bash
# =============================================================================
# check_coverage_single_implementation.sh — one coverage measurement, one rule
# =============================================================================
# Purpose: A gate that measures coverage must not re-implement the measurement.
#          Every direct `cargo llvm-cov` invocation in the repository must be
#          either THE shared implementation or a listed, justified exception.
#
# Coverage: #5581 (coverage thresholds had 5 implementations with 4 different
#          numbers; coverage measurement had 6 implementations, three of them on
#          the v4.1.0 CI path — and two of those three turned "could not
#          measure" into a number that looked normal).
#
# Principle: P1 (有计划必有实现) / ADR-008 Policy 2 (a criterion that is not
#            executed is not a criterion) / "一个规则只有一份实现".
#
# WHY THIS GATE EXISTS
# ---------------------
# #5579 found that the RC/GA gate measured the same eight crates with a
# *different* command, read a different column out of the report, and dropped
# any crate it could not measure from the average — so its number was computed
# over a silently reduced sample. That defect was fixed by hand, in one PR.
#
# A defect that is fixed by hand, once, is a defect that comes back. The only
# durable answer is a gate that says: "if you write `cargo llvm-cov` in a gate
# script, you are writing a second implementation of the measurement — unless
# you are THE implementation, or you can name the reason you are not."
#
# THE RULE
# --------
# A file that **directly invokes** a coverage measurement must be one of:
#
#   * the shared implementation (`scripts/gate/common.sh`), or
#   * listed in `scripts/gate/coverage_measurement_allowlist.txt` with a role
#     and a non-empty reason.
#
# The roles say what the exception IS, so the list can be reviewed:
#
#   the-implementation  — the one shared implementation (exactly one allowed)
#   reporting            — measures, but never decides (prints a WARN)
#   report-generator     — produces a table/artifact for review; not a verdict
#   historical           — a past release's own criterion, off the CI path
#   legacy               — a superseded implementation kept for reference
#
# A file that only **calls** the shared helpers (`coverage_average`,
# `coverage_measure_set`, `coverage_extract_total_percent`) has no direct
# invocation and is therefore not listed at all — that is the intended shape.
#
# Exit codes:
#   0 = PASS — every direct invocation is the shared implementation or listed
#   1 = FAIL — an unlisted direct invocation, a missing reason, a stale entry,
#              or more than one "the-implementation"
#   2 = ENV_ERROR — the allowlist or the tree is missing
#
# Usage:
#   bash scripts/gate/check_coverage_single_implementation.sh [--selftest]
# =============================================================================
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

ALLOWLIST="$SCRIPT_DIR/coverage_measurement_allowlist.txt"

# --- the shared implementation ------------------------------------------------
#
# The one file that is allowed to invoke the measurement tool directly.
SHARED_IMPL="scripts/gate/common.sh"

# --- the scan -----------------------------------------------------------------
#
# ONE definition of what counts as "a direct invocation". The live scan and
# the selftest both run this exact code (they differ only in the tree they
# are pointed at) — a selftest that re-implements the rule it tests proves
# nothing about the rule.
#
# A "hit" is a non-comment line that mentions `cargo llvm-cov` / `cargo-llvm-cov`
# and is not:
#   * a `command -v` probe ("is the tool installed?"),
#   * a `--version` / `-V` probe,
#   * a `cargo install cargo-llvm-cov` (installing the tool is NOT measuring),
#   * an `echo` / `printf` that merely *mentions* the command in prose.
#
# The scan is deliberately file-grained: a file that mentions the tool at all
# must be listed, so a new invocation cannot slip in between two allowlisted
# ones.
#
# NOTE ON THE SINGLE-QUOTE RULE BELOW: the Python source is held in a
# single-quoted shell string, so it must not contain a single quote
# character. Double quotes are used throughout for that reason.
SCAN_PY='import os, re, sys

MENTION = re.compile(r"\bcargo[\s_-]+llvm-cov\b")

SKIP_LINE = re.compile(r"^\s*#")
PROBE = re.compile(r"command\s+-v\b")
VERSION_PROBE = re.compile(r"llvm-cov\s+(--version|-V)\b")
PROSE = re.compile(r"^\s*(echo|printf)\b")
# Installing the tool is not measuring it. This covers both the command
# line (`cargo install cargo-llvm-cov --locked`) and the step title that
# names it (`- name: Install cargo-llvm-cov`) — a CI file that installs the
# tool is not implementing a coverage criterion.
INSTALL = re.compile(r"install", re.IGNORECASE)

# This gate is excluded from its own scan: its text necessarily contains the
# command it looks for (its own selftest fixtures). The selftest is what
# proves the scan works; excluding the rule from its own rule is the same
# shape as `common.sh` being the one listed implementation.
SELF = "scripts/gate/check_coverage_single_implementation.sh"

roots = sys.argv[1:] or ["scripts/gate", "gate", ".gitea/workflows", ".github/workflows"]
SUFFIXES = (".sh", ".yml", ".yaml")

for root in roots:
    if not os.path.isdir(root):
        continue
    for dirpath, _dirnames, filenames in os.walk(root):
        for name in sorted(filenames):
            if not name.endswith(SUFFIXES):
                continue
            path = os.path.join(dirpath, name)
            if path == SELF:
                continue
            try:
                with open(path, encoding="utf-8", errors="replace") as fh:
                    lines = fh.readlines()
            except OSError:
                continue
            hits = []
            for no, line in enumerate(lines, 1):
                if not MENTION.search(line):
                    continue
                if SKIP_LINE.match(line):
                    continue
                if PROBE.search(line):
                    continue
                if VERSION_PROBE.search(line):
                    continue
                if PROSE.match(line):
                    continue
                if INSTALL.search(line):
                    continue
                hits.append(no)
            if hits:
                print("%s\t%s" % (path, ",".join(str(h) for h in hits)))
'

scan_hits() {
    python3 -c "$SCAN_PY" "$@"
}

# --- the allowlist ------------------------------------------------------------

read_allowlist() {
    [ -f "$ALLOWLIST" ] || return 1
    awk -F'\t' '
        /^[[:space:]]*#/ { next }
        /^[[:space:]]*$/ { next }
        {
            if (NF < 3) { print "MALFORMED\t" $0; next }
            print $1 "\t" $2 "\t" $3
        }
    ' "$ALLOWLIST"
}

KNOWN_ROLES="the-implementation reporting report-generator historical legacy"

# --- the check ----------------------------------------------------------------

run_check() {
    local failures=0
    local hits_csv
    hits_csv="$(scan_hits)"

    echo "=== Coverage single-implementation gate (#5581) ==="
    echo "Repo root: $REPO_ROOT"
    echo "Allowlist: ${ALLOWLIST#$REPO_ROOT/}"
    echo

    if [ ! -f "$ALLOWLIST" ]; then
        echo "ENV_ERROR: allowlist not found: $ALLOWLIST"
        return 2
    fi

    # 1. the allowlist itself must be well formed
    local impl_count=0
    local path role reason
    while IFS=$'\t' read -r path role reason; do
        case "$path" in
            MALFORMED*)
                echo "FAIL: malformed allowlist line: $path"
                failures=$((failures+1))
                continue
                ;;
        esac
        if [ -z "$reason" ]; then
            echo "FAIL: allowlist entry '$path' has no reason — an entry without a reason is a hidden problem"
            failures=$((failures+1))
        fi
        if ! printf '%s\n' $KNOWN_ROLES | grep -qx -- "$role"; then
            echo "FAIL: allowlist entry '$path' has unknown role '$role' (known: $KNOWN_ROLES)"
            failures=$((failures+1))
        fi
        if [ "$role" = "the-implementation" ]; then
            impl_count=$((impl_count+1))
        fi
        if [ ! -f "$path" ]; then
            echo "FAIL: allowlist entry '$path' does not exist — a stale exception is not an exception"
            failures=$((failures+1))
        fi
    done < <(read_allowlist)

    if [ "$impl_count" -ne 1 ]; then
        echo "FAIL: exactly one file may be 'the-implementation', found $impl_count"
        failures=$((failures+1))
    fi

    # 2. every direct invocation must be the shared implementation or listed
    local hits_path hits_lines
    while IFS=$'\t' read -r hits_path hits_lines; do
        [ -n "$hits_path" ] || continue
        if [ "$hits_path" = "$SHARED_IMPL" ]; then
            continue
        fi
        if read_allowlist | awk -F'\t' -v p="$hits_path" '$1 == p { found = 1 } END { exit !found }'; then
            echo "PASS: $hits_path (listed; lines $hits_lines)"
        else
            echo "FAIL: $hits_path invokes a coverage measurement directly (lines $hits_lines) but is not listed"
            echo "      Either call the shared implementation in scripts/gate/common.sh, or list the file"
            echo "      in scripts/gate/coverage_measurement_allowlist.txt with a role and a reason."
            failures=$((failures+1))
        fi
    done <<< "$hits_csv"

    # 3. the shared implementation must really be the shared one
    if ! grep -q 'coverage_measure_set()' "$SHARED_IMPL" 2>/dev/null; then
        echo "FAIL: $SHARED_IMPL does not define coverage_measure_set — the shared implementation is missing"
        failures=$((failures+1))
    fi

    echo
    if [ "$failures" -gt 0 ]; then
        echo "FAIL: $failures problem(s) — a second coverage implementation is either unlisted or broken"
        return 1
    fi
    echo "PASS: every direct coverage measurement is the shared implementation or a listed exception"
    return 0
}

# --- selftest -----------------------------------------------------------------
#
# The rule's own fixtures: they must fail closed on an unlisted invocation and
# pass on a listed one. The fixtures are written to a temporary tree, so the
# selftest never depends on the live repository's contents.
#
# The selftest runs the SAME scan code as the live check (`scan_hits` on a
# temporary tree). A selftest with its own copy of the rule would only prove
# that the copy works.
run_selftest() {
    local tmp
    tmp="$(mktemp -d 2>/dev/null || mktemp -d -t covsingle.XXXXXX)" || {
        echo "FAIL: cannot create a temp dir"
        return 1
    }
    local rc=0

    mkdir -p "$tmp/scripts/gate" "$tmp/gate"
    cat > "$tmp/scripts/gate/common.sh" <<'EOF'
coverage_measure_set() { :; }
EOF
    cat > "$tmp/scripts/gate/good_gate.sh" <<'EOF'
#!/usr/bin/env bash
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
coverage_average 80
EOF
    cat > "$tmp/scripts/gate/bad_gate.sh" <<'EOF'
#!/usr/bin/env bash
out=$(cargo llvm-cov test -p sqlrustgo --all-features --tests 2>/dev/null | grep "^TOTAL" | head -1)
echo "$out"
EOF
    cat > "$tmp/gate/listed.sh" <<'EOF'
#!/usr/bin/env bash
cargo llvm-cov --all-features --quiet --output-dir out
EOF
    # Installing the tool is not measuring it — the scan must not flag it.
    cat > "$tmp/gate/installs_only.sh" <<'EOF'
#!/usr/bin/env bash
cargo install cargo-llvm-cov --locked
EOF
    # ... and neither is the step title that names the installation.
    cat > "$tmp/gate/installs_only.yml" <<'EOF'
      - name: Install cargo-llvm-cov
        run: cargo install cargo-llvm-cov --locked
EOF
    printf 'scripts/gate/common.sh\tthe-implementation\tthe one implementation\n' \
        > "$tmp/allow.tsv"
    printf 'gate/listed.sh\treporting\twhole-workspace report only\n' \
        >> "$tmp/allow.tsv"

    local hits
    hits="$(cd "$tmp" && scan_hits scripts/gate gate)"

    case "$hits" in
        *scripts/gate/bad_gate.sh*) : ;;
        *) echo "FAIL: selftest — the fixture that invokes the tool directly was not detected"; rc=1 ;;
    esac
    case "$hits" in
        *gate/listed.sh*) : ;;
        *) echo "FAIL: selftest — the listed fixture was not detected"; rc=1 ;;
    esac
    case "$hits" in
        *scripts/gate/good_gate.sh*) echo "FAIL: selftest — a gate that only CALLS the shared helper was flagged"; rc=1 ;;
        *) : ;;
    esac
    case "$hits" in
        *gate/installs_only.sh*) echo "FAIL: selftest — a file that only INSTALLS the tool was flagged"; rc=1 ;;
        *) : ;;
    esac
    case "$hits" in
        *gate/installs_only.yml*) echo "FAIL: selftest — a step that only INSTALLS the tool was flagged"; rc=1 ;;
        *) : ;;
    esac

    if [ "$rc" -eq 0 ]; then
        echo "PASS: the scan finds a direct invocation, finds a listed one, does not flag a caller of the shared helper, and does not flag a tool installation"
    fi
    rm -rf "$tmp"
    return "$rc"
}

if [ "${1:-}" = "--selftest" ]; then
    run_selftest
    exit $?
fi

run_check
exit $?
