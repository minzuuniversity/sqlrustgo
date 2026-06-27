#!/bin/bash
# clean_fixture_wal.sh - Truncate bloated TPC-H fixture WALs to prevent EAGAIN
# Per AGENTS.md: tests/data/tpch-sf*/sqlrustgo.wal grows on every start_ephemeral.
# After multiple runs, it can hit 1+ GB and cause EAGAIN on the next connection.
# Run before any TPC-H test invocation.

set -uo pipefail

TRUNCATED=0
for f in tests/data/tpch-sf*/sqlrustgo.wal; do
    if [ -f "$f" ]; then
        SIZE=$(stat -f %z "$f" 2>/dev/null || stat -c %s "$f" 2>/dev/null)
        if [ -n "$SIZE" ] && [ "$SIZE" -gt 0 ]; then
            truncate -s 0 "$f"
            echo "[clean_fixture_wal] truncated $f (was ${SIZE} bytes)"
            TRUNCATED=$((TRUNCATED+1))
        fi
    fi
done
[ "$TRUNCATED" -eq 0 ] && echo "[clean_fixture_wal] all fixture WALs already empty" || echo "[clean_fixture_wal] truncated $TRUNCATED file(s)"