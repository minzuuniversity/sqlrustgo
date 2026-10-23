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
