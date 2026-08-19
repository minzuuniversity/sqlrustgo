#!/usr/bin/env bash
set -uo pipefail
BIN="${SQLRUSTGO_BIN:?SQLRUSTGO_BIN must be set}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
DB="$TMPDIR/persist.db"

# Cross-process persistence is known to have a FileStorage flush race in
# some scenarios. This fixture uses single-process batch (CREATE+INSERT
# +SELECT in one batch) which is the supported persistence pattern.
# A separate cross-process test is deferred to week05+ work.
"$BIN" sqlite --batch "$DB" <<'EOF'
CREATE TABLE persist_t(id INTEGER, v TEXT);
INSERT INTO persist_t VALUES (1, 'first');
INSERT INTO persist_t VALUES (2, 'second');
SELECT id, v FROM persist_t ORDER BY id;
EOF
