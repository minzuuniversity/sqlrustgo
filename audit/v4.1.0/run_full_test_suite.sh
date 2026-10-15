#!/usr/bin/env bash
# Full test suite with deterministic result digest (iaia410-manifest/v1,
# SSOT section 5).
#
# Usage: run_full_test_suite.sh <run-id>
#
# Runs `cargo test --all-features` from the repo root. Raw stdout/stderr are
# preserved under traces/ of the run dir (protected by SHA256SUMS). The
# stdout of THIS script is a deterministic result digest:
#   suite=<suite id> :: test result: ... (timing stripped)
#   failure=<test name>
#   final_exit=<exit code>
# Suite ids come from cargo's "Running <x>.rs (path)" stderr headers (in
# execution order); result lines come from stdout. No timing, no absolute
# paths: byte-stable across replays on the same frozen tree, independent of
# build-cache state and CARGO_TARGET_DIR.
set -u
run_id="${1:?usage: run_full_test_suite.sh <run-id>}"
cd "$(git rev-parse --show-toplevel)"
run_dir="audit/v4.1.0/$run_id"
mkdir -p "$run_dir/traces"

raw_out="$run_dir/traces/cargo_test_raw.stdout"
raw_err="$run_dir/traces/cargo_test_raw.stderr"
cargo test --all-features > "$raw_out" 2> "$raw_err"
rc=$?

awk -v final_rc="$rc" -v errfile="$raw_err" '
  BEGIN {
    ns = 0; si = 1
    while ((getline line < errfile) > 0) {
      if (line ~ /^[[:space:]]*Doc-tests /) { suites[++ns] = "doc-tests"; continue }
      if (line ~ /^[[:space:]]*Running /) {
        n = split(line, f, " ")
        name = ""
        for (i = 2; i <= n; i++) if (f[i] ~ /\.rs$/) { name = f[i]; break }
        sub(/\.rs$/, "", name)
        suites[++ns] = name
      }
    }
    close(errfile)
  }
  /^test result: / {
    line = $0
    sub(/; finished in [0-9.]+s$/, "", line)
    print "suite=" suites[si] " :: " line
    si++
  }
  /^---- / && $3 == "stdout" { print "failure=" $2 }
  END { print "final_exit=" final_rc }
' "$raw_out"

exit "$rc"
