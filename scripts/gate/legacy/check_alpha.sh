#!/usr/bin/env bash
# v2.9.0 Alpha Gate — 进入 Alpha 阶段必须通过
#
# #5447: 本脚本**不再**是 CI 发布路径上的门禁。
# `docs/governance/STAGE_CONFIG.yaml` 的 ALPHA 段写明：
#
#   "v2.9.0's check_alpha.sh is hardcoded with v2.9.0 paths and references
#    crates that no longer exist ... Each version provides its own
#    check_alpha_v{VER}.sh that references the version's own worktree paths
#    and gate set."
#
# ⇒ `.gitea/workflows/gate.yml` 的 `legacy-gate` job 现在跑的是
# `scripts/gate/check_alpha_v410.sh`（v4.1.0 专属、有界的真实门禁），
# 不再跑本脚本。本脚本作为 **v2.9.0 版本钉死的历史回归件** 保留在仓库里。
#
# #5416 修的两件事：
#   1. `cargo clippy` 的 `--quiet` 原本写在 `--` 之后 ⇒ 它被当成 **rustc 的
#      参数** ⇒ rustc 报 "Unrecognized option: 'quiet'" ⇒ **每个 crate 都编译
#      失败** ⇒ 这条检查无论代码写成什么样都必然 FAIL。cargo 级选项必须
#      写在 `--` 之前。
#   2. `check()` 原本把被检查命令的输出整个丢掉（`>/dev/null 2>&1`），
#      失败行只有一个词 "FAIL" ⇒ 没有任何可诊断信息。
set -euo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
PASS=0; TOTAL=0; BLOCKERS=0

# 有界执行：CI 的单 runner 上，一次无界的全量测试会占满 2.5 h 的容器预算，
# 把其它 job 全堵在队列里（这正是 legacy-gate 挂死 2.5 h 的原因）。
# GNU coreutils 的 `timeout` 在 Linux runner 上可用；macOS 上是 `gtimeout`。
run_bounded() {
    local secs="$1"; shift
    if command -v timeout >/dev/null 2>&1; then
        timeout "$secs" "$@"
    elif command -v gtimeout >/dev/null 2>&1; then
        gtimeout "$secs" "$@"
    else
        echo "WARN: no timeout(1) available; running unbounded" >&2
        "$@"
    fi
}

check() {
    local name="$1" cmd="$2"
    local out rc
    TOTAL=$((TOTAL+1))
    echo -n "[alpha] $name ... "
    out="$(mktemp)"
    if eval "$cmd" >"$out" 2>&1; then
        echo "PASS"; PASS=$((PASS+1))
    else
        rc=$?
        echo "FAIL (exit $rc)"
        # 失败必须带原因：修前这一行只有一个词 "FAIL"，
        # 「连着三个 FAIL 却看不出所以然」就是这么来的。
        sed -n '1,20p' "$out" | sed 's/^/        /'
        BLOCKERS=$((BLOCKERS+1))
    fi
    rm -f "$out"
}

echo "=== v2.9.0 Alpha Gate (legacy artifact — 不在 CI 发布路径上) ==="
echo ""

check "R4: cargo test --all-features" "run_bounded 1200 cargo test --all-features --quiet"
check "R4: Integration tests 28 files" "bash scripts/test/run_integration.sh --quick"
# #5416: `--quiet` 是 cargo 的选项，必须在 `--` **之前**；
# `--` 之后的一切都是 rustc 的参数。
check "R7: clippy zero warnings" "cargo clippy --all-features --quiet -- -D warnings"
check "R7: cargo fmt" "cargo fmt --check --quiet"
check "A1: SQL Corpus >=85%" "cargo test -p sqlrustgo-sql-corpus --quiet"
check "A3: coverage >=50%" "cargo tarpaulin --ignore-tests --out Json 2>&1 | grep -q '\"coverage\":[5-9][0-9]'"
check "R0: commit binding" 'python3 -c "
import json,subprocess
v=json.load(open(\"verification_report.json\"))
h=subprocess.check_output([\"git\",\"rev-parse\",\"HEAD\"]).decode().strip()
assert v[\"commit\"]==h, \"Commit mismatch\"
print(f\"HEAD={h[:12]} verified\")
"'
check "A4: VERSION_PLAN.md" "test -f docs/releases/v2.9.0/VERSION_PLAN.md"
check "A4: RELEASE_NOTES.md" "test -f docs/releases/v2.9.0/RELEASE_NOTES.md"
check "A4: CHANGELOG.md" "test -f docs/releases/v2.9.0/CHANGELOG.md"
check "A4: FEATURE_MATRIX.md" "test -f docs/releases/v2.9.0/FEATURE_MATRIX.md"
check "A4: INTEGRATION_STATUS.md" "test -f docs/releases/v2.9.0/INTEGRATION_STATUS.md"
check "A4: TEST_PLAN.md" "test -f docs/releases/v2.9.0/TEST_PLAN.md"
check "A4: RELEASE_GATE_CHECKLIST.md" "test -f docs/releases/v2.9.0/RELEASE_GATE_CHECKLIST.md"
check "A4: PERFORMANCE_TARGETS.md" "test -f docs/releases/v2.9.0/PERFORMANCE_TARGETS.md"

echo ""
echo "Alpha Gate: $PASS/$TOTAL passed ($BLOCKERS blockers)"
[ "$BLOCKERS" -eq 0 ] || { echo "BLOCKED"; exit 2; }
echo "PASS — can promote to alpha/v2.9.0"
