#!/usr/bin/env bash
# V312-57 smoke week01/01: `sqlite` 子命令 + --help 可达
# Complement to develop's golden-based fixtures (no .golden file required).
# Asserts: `sqlrustgo sqlite --help` returns 0 and stdout mentions "sqlite"
# plus all major documented flags. NOTE: develop's REPL `.help` is a silent
# no-op, so we exercise the subcommand `--help` flag instead, which is the
# closest stable surface across builds.

set -uo pipefail

BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"

OUT=$("$BIN" sqlite --help 2>&1)
EXIT=$?

if [ "$EXIT" -ne 0 ]; then
    echo "FAIL: exit=$EXIT, got: $OUT"
    exit 1
fi
if ! echo "$OUT" | grep -qi 'sqlite'; then
    echo "FAIL: expected 'sqlite' in --help output, got: $OUT"
    exit 1
fi
# Also assert the major options are documented (i.e. we hit the real --help,
# not a "command not found" stub). Use grep -F -- (fixed strings + end-of-
# options marker) so patterns that start with `--` are not interpreted as
# grep flags.
for opt in --mode --headers --batch --cmd --continue-on-error; do
    if ! echo "$OUT" | grep -F -q -- "$opt"; then
        echo "FAIL: '$opt' missing from --help output, got: $OUT"
        exit 1
    fi
done

echo "PASS: smoke week01/01_help"
exit 0