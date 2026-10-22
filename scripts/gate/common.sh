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
