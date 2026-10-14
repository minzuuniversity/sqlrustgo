#!/usr/bin/env bash
# Focused cargo-mutants run on 8 storage source files.
#
# Avoids files in the #5281 RowId lifecycle freeze zone
# (mvcc_storage, engine, file_storage, wal_record_codec,
# wal_storage, vtu_guard, recovery_engine) — per user instruction
# (2026-10-11) that this audit lane must NOT touch RowId work.
#
# Strategy: cargo-mutants 27 does not support per-file scope. The
# most honest approach is to run the FULL package and post-filter
# the result files to the 8 selected files. The cost is the full
# package compile time (~25-40 min for `sqlrustgo-storage`), then
# per-mutant incremental cost (mostly build-only on already-compiled
# target). Estimated total: 5-8 hours, but we accept the constraint
# that we run on the FULL package and report only the 8-file scope.
#
# Result files: `caught.txt`, `missed.txt`, `timeout.txt`,
# `unviable.txt` — these are NOT file-scoped by cargo-mutants. The
# 8-file filter is applied AFTER the run in a separate analysis step.
#
# Source agent: deepseek-flash (DeepSeek-Harness)
# Source run: audit-storage-mutation-20261011
# HEAD evaluated: 01df92a5d1 (develop/v4.1.0)
set -uo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

MUTANTS_OUT="target/audit-phase2/mutants-out"
mkdir -p "$MUTANTS_OUT"

LOG="$MUTANTS_OUT/run.log"

echo "[$(date -u +%H:%M:%S)] Starting cargo-mutants on full sqlrustgo-storage"
echo "  (post-filter to 8 selected files in analysis step)"
echo "  log: $LOG"

# Run on full package. Output to LOG only (no head pipe that truncates).
./target/cargo-home/bin/cargo-mutants mutants -p sqlrustgo-storage \
  --no-shuffle --baseline skip > "$LOG" 2>&1
RC=$?

echo "[$(date -u +%H:%M:%S)] cargo-mutants exited with RC=$RC"

# Post-filter to 8 files
SELECTED_FILES=(
  "crates/storage/src/replication.rs"
  "crates/storage/src/parallel_wal_storage.rs"
  "crates/storage/src/backup.rs"
  "crates/storage/src/backup_coordinator.rs"
  "crates/storage/src/append_only_storage.rs"
  "crates/storage/src/table_level_storage.rs"
  "crates/storage/src/binary_storage_v2.rs"
  "crates/storage/src/pitr.rs"
)

ALL_LINES=$(( $(wc -l < "$LOG") / 2 ))  # rough estimate
echo ""
echo "Post-filter to 8 files:"
> "$MUTANTS_OUT/8-files-caught.txt"
> "$MUTANTS_OUT/8-files-missed.txt"
> "$MUTANTS_OUT/8-files-timeout.txt"
> "$MUTANTS_OUT/8-files-unviable.txt"

for tier in caught missed timeout unviable; do
  src="$MUTANTS_OUT/${tier}.txt"
  if [ -f "$src" ]; then
    out="$MUTANTS_OUT/8-files-${tier}.txt"
    cnt_total=$(wc -l < "$src")
    cnt_8=0
    for f in "${SELECTED_FILES[@]}"; do
      while IFS= read -r line; do
        if [[ "$line" == "$f"* ]]; then
          echo "$line" >> "$out"
          cnt_8=$((cnt_8+1))
        fi
      done < "$src"
    done
    pct=$(( cnt_total > 0 ? cnt_8 * 100 / cnt_total : 0 ))
    echo "  $tier: total=$cnt_total  in-8-files=$cnt_8  ($pct%)"
  fi
done

echo ""
echo "Done. Mutant lists per category in $MUTANTS_OUT/8-files-*.txt"