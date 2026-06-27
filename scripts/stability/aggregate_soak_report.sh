#!/bin/bash
# aggregate_soak_report.sh - Combine 3-stage SOAK metrics into final report.
#
# Usage:
#   bash scripts/stability/aggregate_soak_report.sh <results_dir> [output_md]
#
# Example:
#   bash scripts/stability/aggregate_soak_report.sh test_results/staged_soak_20260627_120000 \
#        docs/audit/status/SOAK_72H_REPORT.md

set -euo pipefail

RESULTS_DIR="${1:-}"
OUTPUT_MD="${2:-docs/audit/status/SOAK_72H_REPORT.md}"

if [ -z "$RESULTS_DIR" ] || [ ! -d "$RESULTS_DIR" ]; then
    echo "Usage: $0 <results_dir> [output_md]"
    echo "Example: $0 test_results/staged_soak_20260627_120000"
    exit 1
fi

mkdir -p "$(dirname "$OUTPUT_MD")"

cat > "$OUTPUT_MD" << MD
# 72h SOAK Final Report (Issue #3265)

> **Generated**: $(date -u +%Y-%m-%dT%H:%M:%SZ)
> **Source**: $RESULTS_DIR
> **Server**: Z6G4 (192.168.0.252)
> **Spec**: docs/superpowers/specs/2026-06-27-soak-72h-design.md

## Phase Summary

| Phase | Duration | WAL (final) | RSS (final) | FD (final) | Status |
|-------|----------|-------------|-------------|------------|--------|
MD

for phase in phase1_6h phase2_24h phase3_72h; do
    phase_dir="$RESULTS_DIR/$phase"
    metrics="$phase_dir/metrics.csv"

    if [ ! -f "$metrics" ]; then
        echo "| $phase | - | - | - | - | NOT RUN |" >> "$OUTPUT_MD"
        continue
    fi

    duration=$(echo "$phase" | grep -oE '[0-9]+h' | head -1)
    last_wal=$(tail -10 "$metrics" | awk -F, '$1=="wal_mb" {print $2}' | tail -1)
    last_rss=$(tail -10 "$metrics" | awk -F, '$1=="rss_mb" {print $2}' | tail -1)
    last_fd=$(tail -10 "$metrics" | awk -F, '$1=="fd_count" {print $2}' | tail -1)

    status="PASS"
    case "$phase" in
        phase1_6h)
            [ "${last_rss:-0}" -gt 500 ] 2>/dev/null && status="FAIL (RSS)"
            [ "${last_wal:-0}" -gt 1024 ] 2>/dev/null && status="FAIL (WAL)"
            [ "${last_fd:-0}" -gt 100 ] 2>/dev/null && status="FAIL (FD)"
            ;;
        phase2_24h)
            [ "${last_rss:-0}" -gt 1024 ] 2>/dev/null && status="FAIL (RSS)"
            [ "${last_wal:-0}" -gt 1024 ] 2>/dev/null && status="FAIL (WAL)"
            [ "${last_fd:-0}" -gt 150 ] 2>/dev/null && status="FAIL (FD)"
            ;;
        phase3_72h)
            [ "${last_rss:-0}" -gt 1024 ] 2>/dev/null && status="FAIL (RSS)"
            [ "${last_wal:-0}" -gt 100 ] 2>/dev/null && status="FAIL (WAL)"
            [ "${last_fd:-0}" -gt 200 ] 2>/dev/null && status="FAIL (FD)"
            ;;
    esac

    echo "| $phase | $duration | ${last_wal:-?} MB | ${last_rss:-?} MB | ${last_fd:-?} | $status |" >> "$OUTPUT_MD"
done

cat >> "$OUTPUT_MD" << 'MD'

## Issue #3265 Success Criteria

- [ ] **WAL file count**: stabilized after checkpoint (not growing unboundedly)
- [ ] **Memory**: plateaued within 4-6 hours, not growing linearly
- [ ] **Thread count**: stabilized (no thread leaks)
- [ ] **File descriptors**: stabilized (no fd leaks)

## Per-Phase Metrics

### Phase 1: 6h smoke
MD

if [ -f "$RESULTS_DIR/phase1_6h/metrics.csv" ]; then
    echo '```' >> "$OUTPUT_MD"
    head -5 "$RESULTS_DIR/phase1_6h/metrics.csv" >> "$OUTPUT_MD"
    echo '...' >> "$OUTPUT_MD"
    tail -5 "$RESULTS_DIR/phase1_6h/metrics.csv" >> "$OUTPUT_MD"
    echo '```' >> "$OUTPUT_MD"
fi

cat >> "$OUTPUT_MD" << 'MD'

### Phase 2: 24h extended
MD

if [ -f "$RESULTS_DIR/phase2_24h/metrics.csv" ]; then
    echo '```' >> "$OUTPUT_MD"
    head -5 "$RESULTS_DIR/phase2_24h/metrics.csv" >> "$OUTPUT_MD"
    echo '...' >> "$OUTPUT_MD"
    tail -5 "$RESULTS_DIR/phase2_24h/metrics.csv" >> "$OUTPUT_MD"
    echo '```' >> "$OUTPUT_MD"
fi

cat >> "$OUTPUT_MD" << 'MD'

### Phase 3: 72h full
MD

if [ -f "$RESULTS_DIR/phase3_72h/metrics.csv" ]; then
    echo '```' >> "$OUTPUT_MD"
    head -5 "$RESULTS_DIR/phase3_72h/metrics.csv" >> "$OUTPUT_MD"
    echo '...' >> "$OUTPUT_MD"
    tail -5 "$RESULTS_DIR/phase3_72h/metrics.csv" >> "$OUTPUT_MD"
    echo '```' >> "$OUTPUT_MD"
fi

cat >> "$OUTPUT_MD" << MD

## Recommendation

PASS / FAIL / NEEDS_MORE_WORK — to be determined after all phases complete.

## Artifacts

- Raw metrics: $RESULTS_DIR/*/metrics.csv
- Per-phase logs: $RESULTS_DIR/*/soak.log
- Per-phase output: $RESULTS_DIR/*/report/
MD

echo "Report generated: $OUTPUT_MD"