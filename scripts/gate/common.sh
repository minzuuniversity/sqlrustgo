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
make_temp_file() {
    local f
    f="$(mktemp 2>/dev/null)" || f=""
    if [ -z "$f" ] || [ ! -f "$f" ]; then
        echo "ERROR: cannot create a temporary file (mktemp failed) — refusing to continue with an empty path" >&2
        return 1
    fi
    printf '%s' "$f"
}

# A temporary **directory**, with the same contract as `make_temp_file`:
# a bare `mktemp -d` (no template — see above), and a loud failure instead
# of an empty path.
make_temp_dir() {
    local d
    d="$(mktemp -d 2>/dev/null)" || d=""
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
