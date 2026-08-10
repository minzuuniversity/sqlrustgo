#!/usr/bin/env bash
# check_prometheus_metrics.sh — GA-P1 Prometheus Metrics Gate
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PASS=0; FAIL=0
echo "=== GA-P1 Prometheus Metrics Gate ==="
[ -f docs/prometheus-metrics.md ] && echo "  [PASS] prometheus-metrics.md" && PASS=$((PASS+1)) || { echo "  [FAIL] missing"; FAIL=$((FAIL+1)); }
[ -f docs/slow-query-log.md ] && echo "  [PASS] slow-query-log.md" && PASS=$((PASS+1)) || { echo "  [FAIL] missing"; FAIL=$((FAIL+1)); }
[ -x scripts/gate/check_prometheus_metrics.sh ] && echo "  [PASS] gate executable" && PASS=$((PASS+1)) || { echo "  [FAIL] gate"; FAIL=$((FAIL+1)); }
echo "PASS: $PASS, FAIL: $FAIL"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
