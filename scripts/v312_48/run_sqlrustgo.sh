#!/usr/bin/env bash
# V312-48 T2: per-query sqlrustgo runs + SHA256.
#
# Modes:
#   (no env):  full sweep — uses `tee` for the cargo log; will exceed Bash 10-min.
#   TPCH_ONLY_Q=N : run only Q N. Does NOT wipe existing rows; just appends.
#   Q_LIST="1 2 3" : run a sequence of queries. Does NOT wipe existing rows.
#
# In TPCH_ONLY_Q / Q_LIST mode we use `>>` on the cargo log and only refresh the
# SHA256 line for the query we just ran, so a 10-min Bash kill never invalidates
# previously captured TSVs.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
BUNDLE="$ROOT/docs/releases/v3.12.0/perf/TPCH_SF1_CORRECTNESS_BUNDLE"
mkdir -p "$BUNDLE/rows/sqlrustgo" "$BUNDLE/sha256"
export TPCH_SF1_DIR="${TPCH_SF1_DIR:-/tmp/tpch-sf1}"
export TPCH_BINT_DIR="${TPCH_BINT_DIR:-/tmp/tpch-sf1-bin}"
export TPCH_SF1_ROWS_DIR="$BUNDLE/rows/sqlrustgo"
# Avoid panics on known-zero-row queries: see #3653 / #3654 / V312-48-Q8 DBG.
export TPCH_SKIP_PANIC=1

write_sha() {
  local q="$1"   # may be "1", "01", or "q1"; we normalize to zero-padded "q<N>"
  local label="${q#q}"                  # strip leading q if present
  # Zero-pad to 2 digits so q1 -> q01 (matches sqlite_oracle.sh naming).
  label=$(printf "%02d" "$((10#$label))")
  local fname="q${label}.tsv"
  local f="$BUNDLE/rows/sqlrustgo/${fname}"
  local h
  if [[ -s "$f" ]]; then
    h=$(LC_ALL=C sort "$f" | sha256sum | awk '{print $1}')
  else
    h="EMPTY"
  fi
  # Upsert into sha256/sqlrustgo.txt without touching other lines.
  local tmp
  tmp=$(mktemp)
  if [[ -s "$BUNDLE/sha256/sqlrustgo.txt" ]]; then
    grep -v "^q${label} " "$BUNDLE/sha256/sqlrustgo.txt" > "$tmp" || true
  fi
  echo "q${label} ${h}" >> "$tmp"
  LC_ALL=C sort "$tmp" -o "$BUNDLE/sha256/sqlrustgo.txt"
  rm -f "$tmp"
}

run_one() {
  local q="$1"
  echo ">>> running Q${q}" >&2
  TPCH_ONLY_Q="${q}" cargo test --release --test tpch_sf1_22_vs_3engines_test \
    -- --ignored --nocapture >> "$BUNDLE/sqlrustgo_run.log" 2>&1
  # The test writes q<N>.tsv (not zero-padded). Rename to q<N:02>.tsv
  # so file naming matches sqlite_oracle.sh output.
  local label_zero
  label_zero=$(printf "%02d" "$((10#${q#q}))")
  if [[ -f "$BUNDLE/rows/sqlrustgo/q${q}.tsv" && ! -f "$BUNDLE/rows/sqlrustgo/q${label_zero}.tsv" ]]; then
    mv "$BUNDLE/rows/sqlrustgo/q${q}.tsv" "$BUNDLE/rows/sqlrustgo/q${label_zero}.tsv"
  fi
  write_sha "${label_zero}"
}

if [[ -n "${TPCH_ONLY_Q:-}" ]]; then
  run_one "${TPCH_ONLY_Q}"
elif [[ -n "${Q_LIST:-}" ]]; then
  for q in ${Q_LIST}; do
    run_one "${q}"
  done
else
  # Full sweep — must wipe so the per-query TSVs we collect are consistent.
  rm -rf "$BUNDLE/rows/sqlrustgo"
  mkdir -p "$BUNDLE/rows/sqlrustgo"
  cargo test --release --test tpch_sf1_22_vs_3engines_test -- --ignored --nocapture \
    | tee "$BUNDLE/sqlrustgo_run.log"
  > "$BUNDLE/sha256/sqlrustgo.txt"
  for f in "$BUNDLE/rows/sqlrustgo"/q*.tsv; do
    [[ -e "$f" ]] || continue
    q=$(basename "$f" .tsv)
    # Normalize q<N> (no padding) to q<N:02> (zero-padded) for cross-engine consistency.
    q_padded=$(printf "q%02d" "$((10#${q#q}))")
    if [[ "$q" != "$q_padded" ]]; then
      mv "$BUNDLE/rows/sqlrustgo/${q}.tsv" "$BUNDLE/rows/sqlrustgo/${q_padded}.tsv"
    fi
    write_sha "$q_padded"
  done
fi

echo "=== sqlrustgo SHA256 so far ==="
cat "$BUNDLE/sha256/sqlrustgo.txt" 2>/dev/null || echo "(no entries yet)"