#!/usr/bin/env bash
# Process/environment pollution snapshot (iaia410-manifest/v1, SSOT section 5).
#
# Usage: env_pollution_check.sh <run-id> <pre|post>
#
# Writes and cats traces/env_pollution_<phase>.txt inside the run dir.
# Records environment NAMES only (no values) so no secret can leak into
# the manifest evidence.
set -euo pipefail

run_id="${1:-}"
phase="${2:-}"
if [[ ! "$phase" =~ ^(pre|post)$ ]]; then
  echo "usage: env_pollution_check.sh <run-id> <pre|post>" >&2
  exit 2
fi
cd "$(git rev-parse --show-toplevel)"
run_dir="audit/v4.1.0/$run_id"
mkdir -p "$run_dir/traces"
out="$run_dir/traces/env_pollution_$phase.txt"

{
  echo "== env pollution $phase =="
  echo "timestamp_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "hostname=$(hostname)"
  echo
  echo "== sqlrustgo-related processes =="
  ps -eo pid,user,etime,comm,args | grep -iE 'sqlrustgo|soak|mysql-server' | grep -v grep || echo "(none)"
  echo
  echo "== listening TCP ports =="
  ss -ltn
  echo
  echo "== data dirs =="
  found=0
  for d in /tmp/sqlrustgo* /tmp/iaia*; do
    if [[ -e "$d" ]]; then
      found=1
      if [[ -d "$d" ]]; then
        count=$(find "$d" -type f | wc -l)
        mtime=$(stat -c %U "$d" 2>/dev/null || echo unknown)
        echo "$d (dir, files=$count, mtime=$mtime)"
      else
        echo "$d (file, mtime=$(stat -c %U "$d"))"
      fi
    fi
  done
  [[ $found -eq 1 ]] || echo "(none)"
  echo
  echo "== behavior-relevant env var names (names only, no values) =="
  env | grep -E '^(CARGO|RUST|SQLRUSTGO|SOAK|AOCI)' | sed 's/=.*//' || echo "(none)"
  echo
  echo "== cpu/memory pressure =="
  echo "loadavg=$(cat /proc/loadavg)"
  echo "nproc=$(nproc)"
  free -m | head -2
  echo
  echo "== reference DB dataset hash =="
  echo "none (A01 baseline run has no reference dataset)"
} > "$out"
cat "$out"
