#!/usr/bin/env bash
# Regenerate BustubX-EDU B-track oracle .expected files (AB-05 / #5114).
# Oracle = sqlite3 stdout/exit against (seed.sql + exercises/*.sql), per the
# RC-B1 stub recipe ("Generate SQLite oracle via 'sqlite3 < seed.sql'").
# Format: optional STDERR line, EXIT line, then pipe-separated stdout rows.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
B="$ROOT/tests/compat/bustubx_edu_b_track"
command -v sqlite3 >/dev/null || { echo "FAIL-CLOSED: sqlite3 not installed" >&2; exit 1; }
for ex in "$B"/exercises/*.sql; do
  name="$(basename "${ex%.sql}")"
  combined="$(mktemp)"
  cat "$B/seed.sql" "$ex" > "$combined"
  err="$(mktemp)"
  rc=0
  stdout="$(sqlite3 :memory: < "$combined" 2>"$err")" || rc=$?
  {
    if [ -s "$err" ]; then
      echo "STDERR: $(tr '\n' '|' < "$err")"
    fi
    echo "EXIT: $rc"
    echo "--- stdout ---"
    printf '%s\n' "$stdout"
  } > "$B/oracle/${name}.expected"
  rm -f "$combined" "$err"
  echo "generated oracle/${name}.expected"
done
