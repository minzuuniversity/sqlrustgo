#!/usr/bin/env bash
set -uo pipefail
BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

# Successful batch exits 0
"$BIN" sqlite --batch "$TMPDIR/test.db" <<'EOF' >/dev/null 2>&1
SELECT 1;
EOF
echo "exit_success=$?"

# Bad SQL in batch exits 1 (fail-fast)
"$BIN" sqlite --batch "$TMPDIR/test.db" <<'EOF' >/dev/null 2>&1
SELEC 1;
EOF
echo "exit_parse_error=$?"

# Bad SQL with --continue-on-error exits 1 (any failure)
"$BIN" sqlite --batch --continue-on-error "$TMPDIR/test.db" <<'EOF' >/dev/null 2>&1
SELECT 1;
SELEC 2;
EOF
echo "exit_continue_with_error=$?"

# Empty DB arg should still succeed (open then exit)
"$BIN" sqlite --batch "$TMPDIR/test2.db" <<'EOF' >/dev/null 2>&1
SELECT 1;
EOF
echo "exit_empty_db=$?"
