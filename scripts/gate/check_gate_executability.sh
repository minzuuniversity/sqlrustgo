#!/usr/bin/env bash
# check_gate_executability.sh —— 门禁**执行点矩阵**门禁
#
# #5461 / #5450 —— 「阶段清单里的门禁到底有没有人跑」这件事，此前只能靠人读
# workflow 文件去数。本门禁把它变成**可测量、fail-closed** 的判据。
#
# 判据（两条，缺一不可）：
#
#   C1 存在性 —— `STAGE_CONFIG.yaml` 的 `stages.<S>.required_gates` 里写的脚本
#      （`{VER}` 按仓库既有的拼写变体解析）必须落到一个**真实文件**。
#
#   C2 可达性 —— 该脚本必须**真的会被执行**：
#      * 被某个 CI workflow 的 step 直接调用，**或**
#      * 被某个「会被 CI 调用」的门禁间接调用（传递闭包），**或**
#      * 落在某个「CI 里**非 --dry-run** 地、显式钉了 `--stage <S>`」的
#        阶段驱动清单里。
#
# 为什么 `--dry-run` 不算执行点：`check_stage.sh --dry-run` 逐条打印
# `[DRY] <cmd>` 后 `continue` —— **一个门禁都没跑**。改动前 CI 里唯一调用
# 阶段驱动的地方正是 `--dry-run`，于是 GA 的 31 项清单里 12 项从未被执行。
#
# 用法
#   bash scripts/gate/check_gate_executability.sh
#   bash scripts/gate/check_gate_executability.sh --selftest
#   bash scripts/gate/check_gate_executability.sh --json
#
# 退出码：0 = 每一项都有执行点；1 = 有缺失/不可达的门禁（fail-closed）。
#
# 实现：`scripts/gate/gate_executability.py`（**唯一实现**，本文件只是壳）。

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

PY="scripts/gate/gate_executability.py"

if [ ! -f "$PY" ]; then
    echo "FATAL: $PY is missing - the gate cannot be evaluated" >&2
    exit 1
fi

echo "=== Gate Executability Check (#5461 / #5450) ==="
python3 "$PY" "$@"
rc=$?

if [ "$rc" -eq 0 ]; then
    echo
    if [ "${1:-}" = "--selftest" ]; then
        echo "✅ Gate executability selftest PASSED."
    else
        echo "✅ Gate executability check PASSED — every required gate has an execution point."
    fi
else
    echo
    echo "❌ Gate executability check FAILED — a required gate has no execution point."
    echo "   Fix: give it an executor (a CI step, a gate that calls it, or a"
    echo "   non-dry-run stage driver pinned to that stage) — or remove it from"
    echo "   STAGE_CONFIG.yaml if it is not actually required."
fi

exit "$rc"
