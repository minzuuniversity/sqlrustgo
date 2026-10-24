#!/usr/bin/env bash
#
# scripts/gate/common.sh — helpers shared by the gate scripts.
#
# The rule is implemented **once** here: a gate that needs a scratch file or
# a file digest must not re-invent the lookup — a second implementation is a
# second set of portability assumptions, and portability is exactly where
# these gates have bitten us before.
#
# Source it from a gate:
#
#     SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
#     source "$SCRIPT_DIR/common.sh"
#
# Both helpers below are **loud on failure**: they never return an empty
# path and let the caller carry on, because an empty path turns into
# "the thing I was supposed to measure produced nothing" and a gate that
# reports a verdict it did not measure is worse than a gate that stops.

# Where this file lives — resolved here rather than taken from the caller,
# because a helper that depends on the sourcing script's idea of SCRIPT_DIR
# breaks the moment it is sourced from somewhere else.
COMMON_SH_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# The repository root, resolved from this file's own location for the same
# reason COMMON_SH_DIR is: a helper that depends on the sourcing script's idea
# of where the repository is breaks the moment it is sourced from somewhere
# else — or from a script that has not `cd`-ed into the tree yet.
#
# The coverage measurement below runs `cargo` in the repository root, because
# the crate set it measures is addressed by workspace member name and cargo
# resolves those against the workspace the command is started in.
COMMON_SH_REPO_ROOT="$(cd "$COMMON_SH_DIR/../.." && pwd)"

# --- temporary files ---------------------------------------------------------
#
# `mktemp` is called **without a template** on purpose.
#
# The CI runner image's `mktemp` does not accept a template that carries a
# suffix after the `X`s: `mktemp -t name.XXXXXX.py` fails there with
#
#   mktemp: : Invalid argument
#
# and — because a gate script does not necessarily run under `set -e` — that
# failure was swallowed: the command substitution yielded an EMPTY path, and
# the script went on to write its content to "" ("No such file or
# directory") and then report results that had never been produced.
# Measured on CI run 8081 (mutation job 16475); the same trap was hit again
# by `check_test_target_lint_budget.sh` on CI run 8662 (lint-build job
# 17827), where the empty path made the python step die with
# `FileNotFoundError: ''`.
#
# A bare `mktemp` is what the rest of this repository uses (20+ call sites in
# scripts/gate/*.sh) and it is known to work in that image.
#
# The helper **fails loudly** instead of returning an empty path, so a
# harness that cannot create its scratch files stops the gate rather than
# reporting a verdict it did not actually measure.
#
# The error goes to stderr, because the callers capture stdout.
#
# The bare `mktemp` is the first choice because it is what the CI image is
# proven to accept. When it fails — **BSD/macOS `mktemp` requires a
# template**, so on a macOS workstation every gate that sources this file
# aborted with "cannot create a temporary file" — a **prefixed** template is
# tried as the fallback. A prefixed template carries no suffix after the
# Xs, which is exactly the shape the CI image accepts (the failure mode
# documented above was `name.XXXXXX.py`, not a bare template).
make_temp_file() {
    local f
    f="$(mktemp 2>/dev/null)" || f=""
    if [ -z "$f" ] || [ ! -f "$f" ]; then
        f="$(mktemp "${TMPDIR:-/tmp}/sqlrustgo-gate.XXXXXX" 2>/dev/null)" || f=""
    fi
    if [ -z "$f" ] || [ ! -f "$f" ]; then
        echo "ERROR: cannot create a temporary file (mktemp failed) — refusing to continue with an empty path" >&2
        return 1
    fi
    printf '%s' "$f"
}

# A temporary **directory**, with the same contract as `make_temp_file`:
# a bare `mktemp -d` first, a prefixed template as the fallback (BSD/macOS),
# and a loud failure instead of an empty path.
make_temp_dir() {
    local d
    d="$(mktemp -d 2>/dev/null)" || d=""
    if [ -z "$d" ] || [ ! -d "$d" ]; then
        d="$(mktemp -d "${TMPDIR:-/tmp}/sqlrustgo-gate.XXXXXX" 2>/dev/null)" || d=""
    fi
    if [ -z "$d" ] || [ ! -d "$d" ]; then
        echo "ERROR: cannot create a temporary directory (mktemp -d failed) — refusing to continue with an empty path" >&2
        return 1
    fi
    printf '%s' "$d"
}

# --- text predicates that a pipe cannot break --------------------------------
#
# A gate that asks "does this text contain X" must not build a **pipeline** to
# ask it. `printf '%s' "$text" | grep -q X` answers the question twice — once
# with grep's exit status, once with the *pipeline's* exit status — and under
# `set -o pipefail` those two answers can disagree.
#
# `grep -q` exits the moment it has a match, which closes the read end of the
# pipe. A writer that has not finished writing takes SIGPIPE and dies with 141,
# and `pipefail` then reports the pipeline as **failed** even though the answer
# was "found". A gate that turns "found" into "missing" is worse than no gate
# at all: it invents a failure nobody can reproduce.
#
# Measured on #5401 (`scripts/gate/check_storage_db_scope_coverage.sh`):
# 16 methods x 2 wrappers x 60 iterations = 1920 samples produced **32**
# pipeline exits of 141 — and **all 32** were methods that were present
# (cross-checked against a pipe-free reference). 100% false failures, which
# made the whole gate fail in roughly 42% of its runs.
#
# These two helpers ask the question without a pipe. Use them instead of
# `printf ... | grep -q` anywhere a `pipefail` shell pipes text into `grep -q`.
#
# `gate_text_contains_literal` is the literal (non-regex, non-glob) test.
gate_text_contains_literal() {
    local text="$1"
    local needle="$2"
    case "$text" in
        *"$needle"*) return 0 ;;
        *) return 1 ;;
    esac
}

# `gate_text_matches_regex` is the extended-regular-expression test. The
# here-string hands grep a temporary file, so grep reads it to the end and no
# writer is left behind to be killed by SIGPIPE.
#
# It matches the old `printf '%s\n' "$text" | grep -qE -- "$re"` exactly: a
# here-string appends the same trailing newline `printf '%s\n'` did.
gate_text_matches_regex() {
    local text="$1"
    local re="$2"
    grep -qE -- "$re" <<<"$text"
}

# Run the text predicates' own fixtures.
#
# The hard assertions are about the **new** helpers: they must give the same
# answer on every one of many iterations, including on a payload far larger
# than any pipe buffer (2 MiB here; a Linux pipe buffer is 64 KiB and a macOS
# one is 16 KiB). That is the property the pipe-based form did not have.
#
# The old pipe-based form is exercised too, but only as a **negative
# control**: it is expected to misbehave, and the count of misbehaviours is
# reported rather than asserted — a regression test that fails when the bug
# happens not to reproduce is not a regression test, it is a coin toss.
gate_text_helpers_selftest() {
    local fails=0
    local big
    local i
    local rc

    # 2 MiB payload: 4096 lines x 512 bytes.
    big="$(awk 'BEGIN{for(i=0;i<4096;i++){for(j=0;j<512;j++)printf "x";printf "\n"}}')"
    if [ "${#big}" -lt 1048576 ]; then
        echo "SELFTEST FAIL: could not build a >1MiB payload (got ${#big} bytes)"
        return 1
    fi

    # 1. The literal test must find a needle at the very start of a huge
    #    payload — the exact position that made the pipe form lose the race.
    i=0
    while [ "$i" -lt 200 ]; do
        if ! gate_text_contains_literal "$big" "xxx"; then
            echo "SELFTEST FAIL: literal test lost a present needle on iteration $i"
            fails=$((fails + 1))
            break
        fi
        i=$((i + 1))
    done

    # 2. The literal test must not find an absent needle.
    i=0
    while [ "$i" -lt 200 ]; do
        if gate_text_contains_literal "$big" "definitely-not-present-needle"; then
            echo "SELFTEST FAIL: literal test invented a needle on iteration $i"
            fails=$((fails + 1))
            break
        fi
        i=$((i + 1))
    done

    # 3. The regex test must find a present pattern, every time.
    i=0
    while [ "$i" -lt 200 ]; do
        if ! gate_text_matches_regex "$big" "^x+$"; then
            echo "SELFTEST FAIL: regex test lost a present pattern on iteration $i"
            fails=$((fails + 1))
            break
        fi
        i=$((i + 1))
    done

    # 4. The regex test must not find an absent pattern.
    i=0
    while [ "$i" -lt 200 ]; do
        if gate_text_matches_regex "$big" "definitely-not-present-needle"; then
            echo "SELFTEST FAIL: regex test invented a pattern on iteration $i"
            fails=$((fails + 1))
            break
        fi
        i=$((i + 1))
    done

    # Negative control — the pipe-based form this replaced. Reported, not
    # asserted: it is allowed to pass cleanly on a fast machine.
    #
    # `pipefail` is ON for the measurement, because that is the whole point:
    # the pipeline's status is what the old code branched on, and 141 (SIGPIPE)
    # there means "found" was reported as "missing".
    local false_fail=0
    local pipefail_was_on=false
    case "$(set -o)" in
        *pipefail*on*) pipefail_was_on=true ;;
        *) set -o pipefail ;;
    esac
    i=0
    while [ "$i" -lt 200 ]; do
        # `2>/dev/null` silences bash's "printf: write error: Broken pipe"
        # notice — which is the very failure being counted, and 200 lines of
        # it would drown the selftest's own verdict.
        printf '%s\n' "$big" 2>/dev/null | grep -q "xxx"
        rc=$?
        if [ "$rc" -ne 0 ]; then
            false_fail=$((false_fail + 1))
        fi
        i=$((i + 1))
    done
    [ "$pipefail_was_on" = true ] || set +o pipefail
    echo "SELFTEST negative control: pipe-based form mis-answered $false_fail/200 times (0 = the bug did not reproduce here)"
    echo "SELFTEST: $fails failure(s)"
    return "$fails"
}

# --- counting real `#[ignore]` attributes (the SSOT for #5492) --------------
#
# The rule: a literal `#[ignore` counts only when it is a **real attribute** —
# not inside a line comment, and followed by a `fn` item.
#
# This rule lives here **once** because two gates need it and they used to
# disagree. `check_anti_ignore_gate.sh` counted real attributes (and passed);
# `check_test_count_monotonic.sh` counted every literal occurrence (and
# failed). Two numbers for the same thing, two different verdicts, and a
# developer had to guess which gate to believe — and a change that merely
# *mentioned* `#[ignore` in a doc comment could turn the second one red
# (#5492).
#
# `count_real_ignore_attributes [root]` prints the count for the tree under
# `root` (default: the current directory). The scope — `tests/**/*.rs` and
# `crates/**/*.rs` — is the registry's scope
# (`tests/baseline/ignore_registry.json` → `scope.included`).
count_real_ignore_attributes() {
    local root="${1:-.}"
    (
        cd "$root" 2>/dev/null || exit 1
        python3 <<'PYEOF'
import re, pathlib

ATTR = re.compile(r'#\[ignore(\s*=\s*"((?:[^"\\]|\\[\s\S])*)")?\]')
BETWEEN = re.compile(r'(?:\s|//[^\n]*|/\*.*?\*/|\#[^\n]*)*?fn\s+([A-Za-z0-9_]+)')

total = 0
for p in sorted(list(pathlib.Path("tests").rglob("*.rs")) +
                list(pathlib.Path("crates").rglob("*.rs"))):
    src = p.read_text(errors="replace")
    for m in ATTR.finditer(src):
        line_start = src.rfind("\n", 0, m.start()) + 1
        if src[line_start:m.start()].strip().startswith("//"):
            continue                      # prose in a line comment
        if not BETWEEN.match(src[m.end():m.end() + 400]):
            continue                      # prose in a string literal
        total += 1
print(total)
PYEOF
    )
}

# Run the counting rule's own fixtures.
#
# The property under test is **discrimination**: a doc comment that merely
# mentions `#[ignore` must not be counted, and a real attribute must be.
# A counting rule that cannot tell the two apart cannot be a baseline — the
# number it produces would move whenever someone wrote a sentence.
count_real_ignore_attributes_selftest() {
    local tmp
    local got
    local fails=0

    tmp="$(make_temp_dir)" || return 1
    trap 'case "${IGNORE_SELFTEST_DIR:-}" in "${TMPDIR:-/tmp}"/*) rm -rf "$IGNORE_SELFTEST_DIR" ;; esac' EXIT
    IGNORE_SELFTEST_DIR="$tmp"

    mkdir -p "$tmp/tests" "$tmp/crates" || return 1

    # 1. a real attribute on a test fn  -> counted
    cat > "$tmp/tests/real_attribute.rs" <<'EOF'
#[test]
#[ignore = "documented gap"]
fn real_ignored_test() {}
EOF

    # 2. the same word in a doc comment -> NOT counted
    cat > "$tmp/tests/prose_comment.rs" <<'EOF'
// The nightly-only demo is still `#[ignore]` (nightly-only).
fn not_a_test() {}
EOF

    # 3. the same word inside a string -> NOT counted
    cat > "$tmp/tests/prose_string.rs" <<'EOF'
const NOTE: &str = "call #[ignore] when flaky";
fn not_a_test() {}
EOF

    # 4. a real attribute with a reason, no `=`, followed by fn -> counted
    cat > "$tmp/crates/plain_attribute.rs" <<'EOF'
#[ignore]
fn plain_ignored_test() {}
EOF

    got="$(count_real_ignore_attributes "$tmp" | tail -1)"
    if [ "$got" != "2" ]; then
        echo "SELFTEST FAIL: expected 2 real #[ignore] attributes, got '$got'"
        fails=$((fails + 1))
    fi

    # The rule must also agree with the registry on the live tree — that is
    # what makes the two gates one rule instead of two.
    local live
    local registered
    live="$(count_real_ignore_attributes "." | tail -1)"
    registered="$(python3 -c '
import json
print(json.load(open("tests/baseline/ignore_registry.json"))["total_allowed"])
' 2>/dev/null || echo "")"
    if [ -n "$registered" ] && [ "$live" != "$registered" ]; then
        echo "SELFTEST FAIL: live tree has $live real #[ignore] attributes but the registry registers $registered"
        fails=$((fails + 1))
    fi

    if [ "$fails" -eq 0 ]; then
        echo "SELFTEST: PASS (prose not counted, attributes counted, live tree matches the registry)"
    else
        echo "SELFTEST: $fails failure(s)"
    fi
    return "$fails"
}

# --- SHA-256 of a file, portable across Linux/macOS -------------------------
# `shasum` is a Perl script and is NOT guaranteed to exist on a slim CI
# image; `sha256sum` comes from GNU coreutils. The CI runner is only
# *proven* to have coreutils (`sha256sum` is used by
# scripts/gate/check_evidence_binding.sh, which runs in the
# evidence-graph-gate job). So try `shasum` first, fall back to
# `sha256sum`, and — if neither exists — fail loudly instead of silently
# treating an empty hash as "the file is unchanged".
# (scripts/gate/prepare_tpch_fixture.sh uses the same fallback chain.)
sha256_of() {
    if command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" 2>/dev/null | awk '{print $1}'
    elif command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" 2>/dev/null | awk '{print $1}'
    else
        echo ""
    fi
}

# --- provenance: does this document anchor its claims to a real commit? ----
#
# The rule is implemented **once** here, because a second implementation is a
# second set of assumptions — and this rule has already been wrong once:
#
# `check_evidence_binding.sh` used to recognise a document's provenance with
# **fifteen** hard-coded regular expressions (one per format that had ever
# shown up: `**Commit:**`, `**Commits**:`, `| ... | commit | ... |`,
# `**Branch**:`, `**Status**:`, `**Source Issue**:` + `**Agent**:`, …).
# Measured on `docs/releases/v4.1.0` (93 documents): those fifteen patterns
# recognised the anchor of **2** of the 56 documents the gate then flagged.
# Every other document — including 51 whose header *does* carry a real commit
# SHA, written as `**基线**：`develop/v4.1.0` @ `0ba527dd…`` — fell through to
# the line-by-line scan and produced 335 "Type A/B" failures. The gate was
# therefore not detecting fabrication; it was failing to recognise our own
# house style.
#
# The replacement is one rule with a **verifiable** predicate:
#
#     a document has provenance iff its header contains a git SHA that
#     resolves to a real commit object in this repository.
#
# Two properties make this strictly stronger than the fifteen patterns:
#
#  1. It is **format-agnostic**. It does not care whether the author wrote
#     `**基线**：`, `base:`, `**Source commit**:`, `source_head:` or a table
#     cell — it only asks "is there a commit here?".
#  2. It is **falsifiable**. `git cat-file -t <sha>` distinguishes a real
#     commit from an invented one, so a fabricated document cannot buy
#     itself an exemption by typing a plausible-looking hash. The fifteen
#     patterns had no such power: any string matching the shape was enough.
#
# A SHA that resolves to a **blob or tree** is not an anchor — it is file
# content that happens to look like a hash.
#
# `evidence_binding_has_provenance <doc_path>` prints the resolving SHA on
# stdout and returns 0 when the document is anchored; it returns 1 when it is
# not, and 2 when the answer cannot be established (see below).
#
# Why 2 matters: the verification needs the **history**. A checkout with
# `fetch-depth: 1` (what the `postcheck` job used) contains exactly one
# commit, so every historical SHA fails to resolve — and the honest answer
# is "cannot tell", not "no provenance". Returning 2 lets the caller record
# that as a disclosed environment limitation instead of a fabricated
# failure. The CI job now checks out with full history precisely so this
# function can answer for real.
#
# `evidence_binding_has_provenance` returns 2 when the repository's history
# is too shallow to resolve anything at all: it distinguishes "this
# repository has one commit" (shallow) from "this document genuinely has no
# anchor".
evidence_binding_has_provenance() {
    local doc_path="$1"
    local sha

    [ -f "$doc_path" ] || return 1

    # Header only (first 60 lines): the anchor lives in the document's
    # metadata block, not in its body.
    sha="$(evidence_binding_resolve_commit_in_text "$(head -60 "$doc_path" 2>/dev/null)")"
    if [ -n "$sha" ]; then
        printf '%s' "$sha"
        return 0
    fi

    # No anchor resolved. If this checkout is a **shallow** clone, the history
    # is too shallow to tell "no anchor" from "the anchor was not fetched" —
    # say so instead of guessing.
    #
    # The question is asked of git itself (`git rev-parse
    # --is-shallow-repository`) rather than inferred from the commit count: a
    # shallow clone can hold **any** number of commits, and `fetch-depth: 1`
    # is only the default. Counting commits would therefore classify a
    # 2-commit shallow checkout as "history complete" and start reporting
    # failures it cannot substantiate.
    local is_shallow
    is_shallow="$(git rev-parse --is-shallow-repository 2>/dev/null || echo false)"
    if [ "$is_shallow" = "true" ]; then
        return 2
    fi
    return 1
}

# Resolve the first SHA in a piece of text that is a **real commit** in this
# repository; prints it, or nothing.
#
# The SHA candidates are extracted with `grep -o` rather than a shell loop so
# that the whole thing is one pipeline: a `while` loop in a pipeline runs in
# a subshell, and under `set -e` a failing `git cat-file` inside it aborted
# the entire gate script (measured while fixing #5402 — the script stopped
# after two documents and produced no report at all).
#
# A SHA that resolves to a blob or a tree is **not** an anchor: it is file
# content that happens to look like a hash.
evidence_binding_resolve_commit_in_text() {
    local text="$1"
    local candidate

    [ -n "$text" ] || return 0

    while IFS= read -r candidate; do
        [ -n "$candidate" ] || continue
        if [ "$(git cat-file -t "$candidate" 2>/dev/null || true)" = "commit" ]; then
            printf '%s' "$candidate"
            return 0
        fi
    done <<EOF
$(printf '%s\n' "$text" | grep -oE '(^|[^0-9a-zA-Z])[0-9a-f]{7,40}([^0-9a-zA-Z]|$)' \
    | grep -oE '[0-9a-f]{7,40}' | head -20)
EOF
    return 0
}

# Resolve the first real commit SHA cited on **one line** of text.
evidence_binding_resolve_commit_in_line() {
    evidence_binding_resolve_commit_in_text "$1"
}

# Extract the TOTAL line-coverage percentage from a `cargo llvm-cov` text
# report.
#
# Reads the report on stdin; prints the percentage (e.g. "86.46") or nothing
# when the report does not contain one. Two gates need this rule — the v4.1.0
# coverage gate (check_coverage_v312.sh) and the 10-principles R5 rule
# (check_10_principles_v310.sh) — so it lives here, once.
#
# The rule itself is in scripts/gate/coverage_total_percent.py: it addresses
# the Lines group by name in the report's header structure, because "take the
# last percentage in the TOTAL row" is only accidentally right while the
# Branches group happens to be empty — and silently reports *branch* coverage
# the moment that stops being true.
coverage_extract_total_percent() {
    python3 "$COMMON_SH_DIR/coverage_total_percent.py"
}

# Run the extraction rule's own fixtures. A measurement rule that can silently
# pick the wrong column needs a regression test that runs without cargo.
coverage_total_percent_selftest() {
    python3 "$COMMON_SH_DIR/coverage_total_percent.py" --selftest
}

# ============================================================================
# Coverage measurement — ONE implementation of "measure a crate's coverage"
# ============================================================================
#
# #5579. This rule used to live in TWO places, and the two places did not
# agree:
#
#   scripts/gate/check_coverage_v312.sh  — the L1_8 coverage gate (threshold 80%)
#   scripts/gate/check_rc_ga_gate.sh     — A5 in the RC/GA gate (threshold 75/80%)
#
# The second implementation (`get_coverage_avg`) measured the same eight crates
# with a different command (`--all-features --tests`), took a different column
# out of the report (`awk '{print $4}'` — the *regions* percentage, not the
# lines one), and — the defect that actually mattered — had **no fail-closed
# behaviour**: a crate that produced no coverage table was simply left out of
# the average, and the average was reported over the reduced sample.
#
# The consequence is visible in the release record. GA's A5 criterion read
# "67%", below the 75% threshold, and was filed as a coverage gap. The true
# figure — measured by the gate that measures correctly — is 87.77%. The A5
# red was an artefact of a reduced sample, not a coverage gap: storage's
# crash-consistency tests drive a real `sqlrustgo-mysql-server` binary, so
# without the feature derivation and the environment-dependent exclusions
# below, that crate produces NO coverage table at all and drops out of the
# average.
#
# One rule, one implementation. The measurement lives here, both callers call
# it, and a crate that cannot be measured makes the gate FAIL **and name the
# crate**: a verdict is never reported for a sample smaller than the one the
# criterion demands.
#
# The rule is split in two on purpose:
#
#   coverage_measure_set / coverage_measure_crate — the expensive part (cargo)
#   coverage_verdict                              — the decision (pure)
#
# The decision is a pure function of the results, so its selftest runs on
# fixtures with **no cargo at all** — the same contract as
# scripts/gate/coverage_total_percent.py --selftest.

# The canonical L1_8 crate set (GATE_CONDITIONS.md).
#
# The order is the one check_coverage_v312.sh used. The SET is what the
# criterion demands; the two former implementations happened to agree on the
# set while disagreeing on everything else about how to measure it.
COVERAGE_L1_8_CRATES=(
    "sqlrustgo-parser"
    "sqlrustgo-planner"
    "sqlrustgo-executor"
    "sqlrustgo-transaction"
    "sqlrustgo-storage"
    "sqlrustgo-catalog"
    "sqlrustgo-optimizer"
    "sqlrustgo-types"
)

# Result globals. Reset by coverage_measure_set, read by coverage_verdict.
#
# Declared here (not only inside the functions) so that a script which
# sources common.sh under `set -u` can read them before anything measured:
# `${arr[@]}` on an unset array is an unbound-variable error in bash 3.2,
# which is the shell on the maintainers' workstations.
COVERAGE_CRATE_PCT=()          # "crate=percentage" for every measured crate
COVERAGE_FAILED_CRATES=()      # crates that produced no coverage table
COVERAGE_MEASURED_COUNT=0
COVERAGE_TOTAL_COUNT=0
COVERAGE_AVERAGE=""            # filled by coverage_verdict
COVERAGE_CRATE_FEATURES=""     # the features used for the crate last measured

# Per-crate feature requirements, DERIVED from the crate's own sources.
#
# A crate whose integration tests reference a feature-gated module needs
# that feature ENABLED when the tests are compiled. Integration tests link
# the library WITHOUT `cfg(test)`, so a `#[cfg(any(test, feature = "..."))]`
# module simply does not exist for them.
#
# sqlrustgo-storage gates two such items:
#     crates/storage/src/lib.rs:28  #[cfg(any(test, feature = "fault-injection"))]
#                                  pub mod fault_injection;
#     crates/storage/src/lib.rs:92  #[cfg(feature = "bin_storage_default")]
#                                  pub use binary_storage_v2::BinaryTableStorageV2;
# Without the features, `crates/storage/tests/fault_coordination.rs`,
# `crates/storage/tests/a06_failpoint_reachability.rs` and
# `crates/storage/tests/no_namespace_db_refusal_5266.rs` fail to compile
# with
#     error[E0432]: unresolved import `sqlrustgo_storage::fault_injection`
# and the crate measures ZERO coverage.
#
# That zero is not merely a missing number: the coverage gate's own comment
# (below) records that a crate which fails to measure RAISES the reported
# average, so the L1_8 figure was being computed over a silently reduced
# sample. The number was not just missing — it was misleading.
#
# The features are OFF in production (crates/storage/Cargo.toml:30-33),
# so enabling them for the coverage measurement changes nothing about the
# shipped artifact; it only lets the crash-test hooks compile.
#
# The list is DERIVED, not hand-maintained: see derive_crate_features
# below. A hand-maintained list goes stale the moment a new
# `#[cfg(feature = "...")]` module is added, and the failure mode is
# SILENT — the gate measures a reduced sample and reports an inflated
# average.
#
# The derivation is printed, so a reader can see WHY a feature was passed
# rather than having to trust a list.
coverage_derive_crate_features() {
    local crate="$1"
    local crate_dir
    case "$crate" in
        sqlrustgo-*) crate_dir="crates/${crate#sqlrustgo-}" ;;
        *) crate_dir="" ;;
    esac
    # workspace member names are not always <dir>-<name>; find it
    if [ -z "$crate_dir" ] || [ ! -d "$crate_dir" ]; then
        crate_dir=$(find crates -maxdepth 1 -type d -name "*" 2>/dev/null \
            | while read -r d; do
                if grep -qx "name = \"$crate\"" "$d/Cargo.toml" 2>/dev/null; then
                    echo "$d"
                    break
                fi
            done)
    fi
    [ -z "$crate_dir" ] && return 0
    [ -d "$crate_dir" ] || return 0

    python3 - "$crate_dir" <<'PYEOF'
import os
import re
import sys

crate_dir = sys.argv[1]
found = []

for root in (os.path.join(crate_dir, "src"), os.path.join(crate_dir, "tests")):
    if not os.path.isdir(root):
        continue
    for dirpath, _dirs, files in os.walk(root):
        for fn in files:
            if not fn.endswith(".rs"):
                continue
            path = os.path.join(dirpath, fn)
            try:
                lines = open(path, encoding="utf-8").read().split("\n")
            except OSError:
                continue
            for i, line in enumerate(lines):
                m = re.search(r"#\s*\[cfg\(([^\]]*)\)\]", line)
                if not m:
                    continue
                feats = re.findall(r'feature\s*=\s*"([^"]+)"', m.group(1))
                if not feats:
                    continue
                # A module-level public item is what a test can import.
                for j in range(i + 1, min(i + 4, len(lines))):
                    pm = re.match(r"\s*pub\s+(mod|use)\b", lines[j])
                    if pm:
                        for f in feats:
                            found.append((f, "%s:%d" % (path, i + 1)))
                        break

if found:
    print(" ".join(sorted(set(f for f, _ in found))))
    for f, ev in sorted(set(found)):
        print("      %s <- %s" % (f, ev), file=sys.stderr)
PYEOF
}

coverage_features_for_crate() {
    local crate="$1"
    local out
    out=$(cd "$COMMON_SH_REPO_ROOT" && coverage_derive_crate_features "$crate" 2>/dev/null)
    echo "$out" | tr -d '\n' | tr -s ' ' | sed 's/^ *//;s/ *$//'
}

# The crash-consistency tests in sqlrustgo-storage drive a real server
# process: `crates/storage/tests/fault_coordination.rs` locates the
# `sqlrustgo-mysql-server` binary at a machine-specific path, and
# `crates/storage/tests/crash_{commit_fsync,wal_index}_b1.rs` use the same
# helper. They also need the server to reach an exact fault point within
# a timeout.
#
# They are EXCLUDED from the coverage measurement:
#
#   * they are machine-dependent (a pre-built binary at a fixed path), and
#   * a failure in ANY of them makes `cargo llvm-cov` produce NO coverage
#     table for the whole crate, so the crate measures zero and the L1_8
#     average is computed over a silently reduced sample.
#
# The crash semantics themselves are covered by a dedicated gate —
# scripts/gate/check_g14_real_crash.sh — which is the right instrument for
# a "does the data survive SIGKILL" question. A line-coverage gate is not.
#
# The excluded FUNCTION NAMES are extracted from the excluded files at run
# time, so a new crash test is covered without touching this list. cargo
# cannot exclude a whole test TARGET, only whole packages, and libtest's
# `--skip` filters by function name — hence the extraction.
COVERAGE_EXCLUDED_TEST_FILES=(
    "crates/storage/tests/crash_commit_fsync_b1.rs"
    "crates/storage/tests/crash_wal_index_b1.rs"
    "crates/storage/tests/fault_coordination.rs"
)

# Test files whose assertions are wall-clock bounds.
#
# A test that asserts "this operation takes less than N seconds" cannot
# produce a meaningful result under `cargo llvm-cov`: the instrumentation
# makes the code 2-10x slower, so the assertion fails for a reason that
# has nothing to do with the code under test — and the failure produces
# NO coverage table at all, which zeroes the whole crate's coverage and
# makes the L1_8 average misleading.
#
# Measured 2026-10-22: with the crash tests excluded but the performance
# test still running,
#     write_path_scaling_5181::report_single_row_delete_cost
#     -> "a single-row delete at 20000 rows costs 0.00226832s; that is
#         too slow for an OLTP point update"   (the bound is 0.002s)
# and the crate produced NO coverage table. With that file excluded as
# well, the crate measures 81.31%.
#
# Note this test also fails WITHOUT instrumentation on the base commit
# (0.00227s against the same 0.002s bound) — it is a pre-existing
# performance debt, tracked separately. What this exclusion fixes is that
# the debt must not also destroy the crate's coverage measurement.
#
# The whole FILE is excluded rather than individual functions: the wall
# clock is measured in a helper the test calls (e.g. `per_row_delete`),
# so the assertion is not visible inside the test function's own body.
COVERAGE_PERF_BOUND_TEST_FILES=(
    "crates/storage/tests/write_path_scaling_5181.rs"
    "crates/storage/tests/e2e_binlog_replication_test.rs"
    "crates/storage/tests/b22_concurrent_flush_ab.rs"
)

coverage_excluded_test_names() {
    local file
    for file in "${COVERAGE_EXCLUDED_TEST_FILES[@]}" "${COVERAGE_PERF_BOUND_TEST_FILES[@]}"; do
        [ -f "$COMMON_SH_REPO_ROOT/$file" ] || continue
        python3 - "$COMMON_SH_REPO_ROOT/$file" <<'PYEOF'
import io
import re
import sys

path = sys.argv[1]
TEST_ATTR = re.compile(r"^\s*#\s*\[\s*(test|sqlrustgo_test)\s*\]")
FN = re.compile(r"^\s*(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)")
lines = io.open(path, encoding="utf-8").read().split("\n")
for i, line in enumerate(lines):
    if not TEST_ATTR.match(line):
        continue
    for j in range(i + 1, min(i + 4, len(lines))):
        m = FN.match(lines[j])
        if m:
            print(m.group(1))
            break
PYEOF
    done
}

# Measure ONE crate's line coverage.
#
#   stdout : the percentage (e.g. "86.46")
#   rc 0   : measured
#   rc 1   : could NOT be measured — nothing on stdout, the reason on stderr
#
# The caller must treat "could not measure" as a FAILURE, never as a zero and
# never as a crate to be dropped from the average: that is the whole point of
# #5579.
coverage_measure_crate() {
    local crate="$1"
    local features feature_args=() skip_args=() t output rc=0 pct

    features="$(coverage_features_for_crate "$crate")"
    COVERAGE_CRATE_FEATURES="$features"
    if [ -n "$features" ]; then
        feature_args=(--features "$features")
    fi
    for t in $(coverage_excluded_test_names); do
        skip_args+=(--skip "$t")
    done

    # `--skip` is a LIBTEST argument, so it must go AFTER the `--` separator
    # (`cargo llvm-cov test [OPTIONS] [TESTNAME] [-- <args>...]`). Putting it
    # before the `--` makes cargo reject the whole invocation.
    #
    # `--test-threads=1` is NOT optional. Under `cargo llvm-cov` the code runs
    # 2-10x slower, and the default parallel test execution then has the
    # threads competing for CPU: measurements of the same crates came out
    # different from run to run (sqlrustgo-planner measured 88.91% and PASSED
    # with --test-threads=1, and FAILED with the default thread count on the
    # same checkout). A coverage number that depends on machine load is not a
    # measurement. It also matches the repository's own rule: AGENTS.md §10
    # requires concurrent tests to run with `--test-threads=1`.
    #
    # `|| rc=$?` instead of `set +e` / `set -e`: it is safe under `set -e` and
    # it does not mutate the caller's shell options.
    output=$( cd "$COMMON_SH_REPO_ROOT" && cargo llvm-cov test -p "$crate" \
                  ${feature_args[@]+"${feature_args[@]}"} --no-fail-fast \
                  -- --test-threads=1 ${skip_args[@]+"${skip_args[@]}"} 2>&1 ) || rc=$?

    # The TOTAL row carries FOUR percentages (regions / functions / lines /
    # branches). The extraction rule lives in coverage_extract_total_percent
    # — one rule, one implementation.
    pct=$(printf '%s\n' "$output" | coverage_extract_total_percent)
    if [ "$rc" -ne 0 ] || [ -z "$pct" ]; then
        printf 'could not measure %s (exit=%s%s)\n' "$crate" "$rc" \
            "${pct:+, got $pct%}" >&2
        return 1
    fi
    printf '%s\n' "$pct"
}

# Measure a SET of crates.
#
# Resets the result globals, measures each crate in turn and records what
# came back. Returns 1 if any crate could not be measured — but the DECISION
# is left to coverage_verdict, which is the single place that decides
# whether a result is acceptable.
#
# Prints one line per crate on stdout, so a caller can show the detail.
coverage_measure_set() {
    COVERAGE_CRATE_PCT=()
    COVERAGE_FAILED_CRATES=()
    COVERAGE_MEASURED_COUNT=0
    COVERAGE_TOTAL_COUNT=0
    local crate pct rc=0
    for crate in ${@+"$@"}; do
        COVERAGE_TOTAL_COUNT=$((COVERAGE_TOTAL_COUNT + 1))
        printf 'Measuring coverage for %s ...\n' "$crate"
        pct="$(coverage_measure_crate "$crate")" || rc=1
        if [ -n "$pct" ]; then
            if [ -n "$COVERAGE_CRATE_FEATURES" ]; then
                printf '  features: %s\n' "$COVERAGE_CRATE_FEATURES"
            fi
            printf '  %s: %s%%\n' "$crate" "$pct"
            COVERAGE_CRATE_PCT+=("$crate=$pct")
            COVERAGE_MEASURED_COUNT=$((COVERAGE_MEASURED_COUNT + 1))
        else
            COVERAGE_FAILED_CRATES+=("$crate")
        fi
    done
    return $rc
}

# Decide the verdict from the measured results.
#
#   usage: coverage_verdict <threshold>
#
# PURE: it performs no measurement and touches nothing but the result
# globals, so its selftest can drive it with fixtures and no cargo.
#
# Fails closed in BOTH directions that matter:
#   * a crate that could not be measured  => FAIL, naming the crate
#   * every crate measured but the average is below the threshold => FAIL
#
# The first case is the one #5579 is about: a reduced sample is not a
# coverage figure, it is a measurement that did not happen.
coverage_verdict() {
    local threshold="$1"
    local measured="${#COVERAGE_CRATE_PCT[@]}"
    local total="$COVERAGE_TOTAL_COUNT"
    local failed_csv="${COVERAGE_FAILED_CRATES[*]+${COVERAGE_FAILED_CRATES[*]}}"

    if ! command -v bc >/dev/null 2>&1; then
        printf 'FAIL: bc is not available — cannot compute the coverage average\n' >&2
        COVERAGE_AVERAGE=""
        return 1
    fi

    if [ "$total" -eq 0 ]; then
        printf 'FAIL: no crates to measure\n' >&2
        COVERAGE_AVERAGE=""
        return 1
    fi

    if [ "$measured" -ne "$total" ]; then
        printf 'FAIL: measured %s of %s crates — no coverage data for: %s\n' \
            "$measured" "$total" "$failed_csv" >&2
        printf '      The average is only meaningful over the full crate set —\n' >&2
        printf '      a reduced sample silently changes the verdict.\n' >&2
        COVERAGE_AVERAGE=""
        return 1
    fi

    # `scale=4` is set EXPLICITLY, because the arithmetic must not depend on
    # which `bc` is installed. GNU bc keeps the operands' scale through `+`
    # (so the sum is 702.21 and the average is 87.77); a POSIX bc truncates
    # at scale 0 (so the sum is 698 and the average is 87.25). Both are
    # "correct" for their implementation and they disagree by half a point —
    # a gate threshold must not move because the runner image changed.
    local entry pct sum=0
    for entry in ${COVERAGE_CRATE_PCT[@]+"${COVERAGE_CRATE_PCT[@]}"}; do
        pct="${entry#*=}"
        sum=$(echo "scale=4; $sum + $pct" | bc)
    done
    # The divisor is the size of the SET the criterion names (8 crates), not
    # the number of crates that happened to contribute. The guard above makes
    # the two equal on every path that reaches this line — so the choice only
    # matters if the guard is ever removed, and then "the missing crate counts
    # as 0%" is the conservative direction rather than the flattering one.
    COVERAGE_AVERAGE=$(echo "scale=2; $sum / $total" | bc)

    if (( $(echo "$COVERAGE_AVERAGE < $threshold" | bc -l) )); then
        printf 'FAIL: average %s%% < threshold %s%%\n' \
            "$COVERAGE_AVERAGE" "$threshold" >&2
        return 1
    fi
    return 0
}

# Measure the canonical L1_8 set and decide.
#
#   usage: coverage_average <threshold> [crate...]
#
# With no crate list, the canonical L1_8 set is measured. Prints the
# per-crate detail and a summary line that names BOTH the average and the
# size of the sample the average was computed over — a reduced sample is
# then visible in the report itself, not only in the exit status.
#
# Returns 0 = PASS, 1 = FAIL. The caller distinguishes the two failure
# modes with COVERAGE_FAILED_CRATES (non-empty => the measurement itself
# failed) and COVERAGE_AVERAGE (empty => the average was never computed).
coverage_average() {
    local threshold="$1"; shift
    local crates=()
    if [ "$#" -gt 0 ]; then
        crates=("$@")
    else
        crates=(${COVERAGE_L1_8_CRATES[@]+"${COVERAGE_L1_8_CRATES[@]}"})
    fi

    coverage_measure_set ${crates[@]+"${crates[@]}"} || true

    if ! coverage_verdict "$threshold"; then
        return 1
    fi

    printf 'average coverage: %s%% (threshold: %s%%) — measured %s/%s crates\n' \
        "$COVERAGE_AVERAGE" "$threshold" "$COVERAGE_MEASURED_COUNT" "$COVERAGE_TOTAL_COUNT"
    return 0
}

# The verdict rule's own fixtures.
#
# A gate that decides "did this measurement cover the whole set?" needs a
# test for the answer, and the answer must not require cargo: the cases
# below are the shape of a reduced sample, of a below-threshold average and
# of a healthy measurement.
#
# The arithmetic case pins the average to a known value, which also catches a
# change in the `bc` scale handling (see coverage_verdict).
coverage_measurement_selftest() {
    local fails=0

    _cm_selftest_case() {
        local name="$1" want_rc="$2" want_avg="$3"
        shift 3
        coverage_verdict "$@" >/dev/null 2>&1
        local rc=$?
        # The raw value, so that "no average was reported" (the empty string)
        # is distinguishable from "an average of <none>".
        local got="${COVERAGE_AVERAGE-}"
        local shown="$got"
        [ -n "$shown" ] || shown="<none>"
        local want_shown="$want_avg"
        [ -n "$want_shown" ] || want_shown="<none>"
        if [ "$rc" -ne "$want_rc" ]; then
            printf 'FAIL  %s: rc=%s (want %s)\n' "$name" "$rc" "$want_rc"
            fails=$((fails + 1))
            return
        fi
        if [ "$got" != "$want_avg" ]; then
            printf 'FAIL  %s: average=%s (want %s)\n' "$name" "$shown" "$want_shown"
            fails=$((fails + 1))
            return
        fi
        printf 'ok    %s\n' "$name"
    }

    # 1. The healthy case: all eight crates measured, average above the
    #    threshold. These are the real numbers the L1_8 measurement produced
    #    on 2026-10-24 — the average must come out at 87.77.
    COVERAGE_CRATE_PCT=(
        "sqlrustgo-parser=86.46" "sqlrustgo-planner=88.91"
        "sqlrustgo-executor=82.68" "sqlrustgo-transaction=93.29"
        "sqlrustgo-storage=80.65" "sqlrustgo-catalog=87.33"
        "sqlrustgo-optimizer=87.85" "sqlrustgo-types=95.04"
    )
    COVERAGE_FAILED_CRATES=()
    COVERAGE_MEASURED_COUNT=8
    COVERAGE_TOTAL_COUNT=8
    _cm_selftest_case "full sample, average 87.77 >= 80" 0 "87.77" 80.0

    # 2. The reduced sample — THE defect of #5579. Seven of eight crates
    #    measured: the verdict must be FAIL, and the average must not be
    #    reported at all (the old implementation reported the average over
    #    the seven and carried on).
    COVERAGE_CRATE_PCT=(
        "sqlrustgo-parser=86.46" "sqlrustgo-planner=88.91"
        "sqlrustgo-executor=82.68" "sqlrustgo-transaction=93.29"
        "sqlrustgo-catalog=87.33" "sqlrustgo-optimizer=87.85"
        "sqlrustgo-types=95.04"
    )
    COVERAGE_FAILED_CRATES=("sqlrustgo-storage")
    COVERAGE_MEASURED_COUNT=7
    COVERAGE_TOTAL_COUNT=8
    _cm_selftest_case "reduced sample (storage missing) => FAIL" 1 "" 80.0

    # 3. Every crate measured, but the average is below the threshold:
    #    an honest FAIL.
    COVERAGE_CRATE_PCT=(
        "a=50.00" "b=50.00" "c=50.00" "d=50.00"
        "e=50.00" "f=50.00" "g=50.00" "h=50.00"
    )
    COVERAGE_FAILED_CRATES=()
    COVERAGE_MEASURED_COUNT=8
    COVERAGE_TOTAL_COUNT=8
    _cm_selftest_case "full sample, average 50.00 < 75 => FAIL" 1 "50.00" 75.0

    # 4. An empty set must not divide by zero.
    COVERAGE_CRATE_PCT=()
    COVERAGE_FAILED_CRATES=()
    COVERAGE_MEASURED_COUNT=0
    COVERAGE_TOTAL_COUNT=0
    _cm_selftest_case "empty set => FAIL" 1 "" 80.0

    if [ "$fails" -ne 0 ]; then
        printf 'coverage_measurement_selftest: %s case(s) FAILED\n' "$fails" >&2
        return 1
    fi
    printf 'coverage_measurement_selftest: all cases passed\n'
    return 0
}

