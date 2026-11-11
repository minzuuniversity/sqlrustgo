#!/usr/bin/env bash
# Heartbeat wrapper for soak arms in evidence manifests (AB-07 / #5115).
#
# Runs an arbitrary soak command under supervision:
#   1. starts it in the background and captures the REAL shell pid,
#   2. runs soak_heartbeat.py against that pid (JSONL: start/beat/exit),
#   3. waits for the soak, then runs check_soak_heartbeat.py (fail-closed).
# Exit code: 0 only when BOTH the soak command and the heartbeat check pass;
# otherwise 1 (soak failure and heartbeat violation are reported distinctly).
#
# Usage:
#   HB_OUT=/tmp/hb.jsonl [HB_MAX_GAP=90] [HB_INTERVAL=15] \
#     bash scripts/soak/run_soak_with_heartbeat.sh <soak command...>
# Example (AB-07 1h arm):
#   HB_OUT=/tmp/ab07-hb.jsonl bash scripts/soak/run_soak_with_heartbeat.sh \
#     bash scripts/soak/v400_1h_soak.sh 60 3412
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HB_OUT="${HB_OUT:?HB_OUT required (heartbeat JSONL path)}"
HB_INTERVAL="${HB_INTERVAL:-15}"
HB_MAX_GAP="${HB_MAX_GAP:-90}"

if [ "$#" -lt 1 ]; then
  echo "usage: HB_OUT=<file> $0 <soak command...>" >&2
  exit 2
fi

"$@" &
SOAK_PID=$!
python3 "$SCRIPT_DIR/soak_heartbeat.py" --pid "$SOAK_PID" --out "$HB_OUT" --interval "$HB_INTERVAL" &
HB_PID=$!
wait "$SOAK_PID"; SOAK_RC=$?
wait "$HB_PID" 2>/dev/null || true

python3 "$SCRIPT_DIR/check_soak_heartbeat.py" --file "$HB_OUT" --max-gap "$HB_MAX_GAP"
HB_RC=$?

echo "soak_rc=$SOAK_RC hb_rc=$HB_RC hb=$HB_OUT"
if [ "$SOAK_RC" -eq 0 ] && [ "$HB_RC" -eq 0 ]; then
  echo "AB-07 supervised soak: PASS"
  exit 0
fi
echo "AB-07 supervised soak: FAIL (soak_rc=$SOAK_RC hb_rc=$HB_RC)"
exit 1
