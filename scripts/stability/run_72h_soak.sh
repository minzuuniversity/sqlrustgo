#!/bin/bash
# run_72h_soak.sh - G13 72h 真实长跑 (Post-GA Nightly)
#
# 这是 Post-GA 持续监控, GA 不卡此 gate.
# 实际跑: 每周 1 次 Nightly CI.
#
# Refs: V390_TEST_PLAN_ROUND2_REVIEW §G13 (72h 推迟到 Post-GA)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

HOURS=${HOURS:-72}
INTERVAL=${INTERVAL:-120}  # 72h 较稀疏监控
RESULTS_DIR=${RESULTS_DIR:-"test_results/stability_72h_$(date +%Y%m%d_%H%M%S)"}

# 复用 24h 脚本 (大部分逻辑相同)
HOURS=$HOURS INTERVAL=$INTERVAL RESULTS_DIR=$RESULTS_DIR \
    bash "$SCRIPT_DIR/run_24h_soak.sh"
