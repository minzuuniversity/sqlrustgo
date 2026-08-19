#!/usr/bin/env bash
set -uo pipefail
BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
"$BIN" sqlite --batch "$TMPDIR/test.db" <<'EOF'
SELECT 1 AS x;
EOF
