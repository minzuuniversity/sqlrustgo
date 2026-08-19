#!/usr/bin/env bash
set -uo pipefail
BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
"$BIN" sqlite --batch "$TMPDIR/test.db" <<'EOF'
CREATE TABLE w2_t(id INTEGER, name TEXT);
INSERT INTO w2_t VALUES (1, 'Alice');
INSERT INTO w2_t VALUES (2, 'Bob');
SELECT id, name FROM w2_t ORDER BY id;
EOF
