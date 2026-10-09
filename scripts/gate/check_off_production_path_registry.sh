#!/usr/bin/env bash
#
# check_off_production_path_registry.sh — 「不在生产执行路径」排除清单防腐化检查
#
# 背景（#5193 / IAIA-410 F3，路线裁决 = 选项 C）:
#   B2 门禁曾把 crates/planner 的 3 个 test binary 计入通过率，但该 crate 的
#   物理计划层无 execute()、且无任何生产调用方 —— 套件全绿也证明不了任何
#   用户可观测行为。这类排除与 DISABLED（失败）、PERF（慢）都不同，故单列。
#
# 本脚本的职责（对应 registry 文档 §3 的 R1/R2/R3）:
#   R1  planner 物理计划层是否仍无 execute() —— 若已实现，排除理由消失
#   R2  生产侧是否仍不 import sqlrustgo_planner —— 若已接入，排除理由消失
#   R3  登记是否过期（stale_after_days 天内未复审）
#   R4  登记的 binary 是否真的不存在生产引擎引用（防止登记与代码脱节）
#   R5  #5193 是否仍 open —— 若已按 A/B 路线变更，登记需重写
#
# 退出码: 0 = PASS, 1 = FAIL（阻断合并）, 2 = 用法错误
#
# 依据 docs/releases/v4.1.0/b2-off-production-path-registry.md

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

REGISTRY="docs/releases/v4.1.0/b2-off-production-path-registry.md"
RUNNER="scripts/gate/run_b2_per_binary.py"
STALE_AFTER_DAYS="${STALE_AFTER_DAYS:-90}"

PASS_COUNT=0
FAIL_COUNT=0
FAIL_ITEMS=()

add_pass() { echo "  [PASS] $1"; PASS_COUNT=$((PASS_COUNT + 1)); }
add_fail() { echo "  [FAIL] $1"; FAIL_ITEMS+=("$1"); FAIL_COUNT=$((FAIL_COUNT + 1)); }

echo "=== check_off_production_path_registry.sh (#5193 option C) ==="
echo "registry : $REGISTRY"
echo "stale_after_days: $STALE_AFTER_DAYS"
echo

# ---- 前置：登记文档与 runner 存在 ----------------------------------------
if [ ! -f "$REGISTRY" ]; then
    echo "  [FAIL] registry missing: $REGISTRY"
    echo "FAIL: 登记文档不存在，无法验证排除项是否仍然成立"
    exit 1
fi
if [ ! -f "$RUNNER" ]; then
    echo "  [FAIL] runner missing: $RUNNER"
    exit 1
fi
echo "  [PASS] registry and runner both present"
echo

# ---- R1: planner 物理计划层是否仍无 execute() ----------------------------
echo "--- R1: PhysicalPlan trait still has no execute() ---"
if [ ! -f crates/planner/src/physical_plan.rs ]; then
    add_fail "R1 crates/planner/src/physical_plan.rs not found — cannot verify"
else
    # 只看 trait 定义块（`pub trait PhysicalPlan {` 到其闭合 `}`）内的方法。
    # 全文件 grep 'fn execute' 会误报：SeqScanExec 有自己的固有方法
    # execute()（physical_plan.rs:59），它硬编码 Ok(vec![]) 且同样不在生产
    # 路径上，但它不是 trait 的一部分 —— 判据必须是「trait 是否新增了
    # execute()」，即选项 A 是否开工，而不是「文件里有没有 execute 字样」。
    R1_EXEC="$(python3 -c "
import re,sys
src=open(sys.argv[1]).read()
m=re.search(r'pub trait PhysicalPlan\b[^{]*\{(.*?)\n\}', src, re.S)
if not m:
    print('NOTRAIT'); sys.exit(0)
print('HAS_EXEC' if re.search(r'\bfn execute\b', m.group(1)) else 'NO_EXEC')
" crates/planner/src/physical_plan.rs)"
    case "$R1_EXEC" in
        NO_EXEC)
            add_pass "R1 PhysicalPlan trait still has no execute() — exclusion reason holds" ;;
        HAS_EXEC)
            add_fail "R1 PhysicalPlan trait now defines execute() — #5193 option A is \
under way; this registry must be revisited (re-evaluate whether these binaries \
belong in the gate)" ;;
        *)
            add_fail "R1 could not locate 'pub trait PhysicalPlan' block in \
crates/planner/src/physical_plan.rs — cannot verify the exclusion reason" ;;
    esac
fi
echo

# ---- R2: 生产侧是否仍不 import planner -----------------------------------
echo "--- R2: no production entry point imports sqlrustgo_planner ---"
# 入口范围：mysql-server 与根 crate 的引擎实现文件。
R2_HITS="$(grep -rn 'sqlrustgo_planner' \
    crates/mysql-server/src/ \
    src/engine_select.rs src/engine_dml.rs src/execution_engine.rs \
    2>/dev/null || true)"
if [ -n "$R2_HITS" ]; then
    add_fail "R2 production code references sqlrustgo_planner — planner may now be \
wired in; re-evaluate this registry. Hits: $(echo "$R2_HITS" | head -3 | tr '\n' ' ')"
else
    add_pass "R2 no sqlrustgo_planner reference in production entry points"
fi
echo

# ---- R3: 登记是否过期 ----------------------------------------------------
echo "--- R3: registry freshness ---"
# 用登记文档的 mtime 作为复审时点：任何对清单的改动都刷新它，
# 从而「改文档即复审」，避免靠 git log 猜上一次实质复核时间。
MTIME_EPOCH="$(python3 -c "
import os,sys,time
p=sys.argv[1]
print(int(os.path.getmtime(p)))
" "$REGISTRY")"
AGE_DAYS=$(( ( $(date +%s) - MTIME_EPOCH ) / 86400 ))
if [ "$AGE_DAYS" -gt "$STALE_AFTER_DAYS" ]; then
    add_fail "R3 registry last reviewed ${AGE_DAYS}d ago (> ${STALE_AFTER_DAYS}d) — \
stale exclusion. Re-verify R1/R2 and update the registry, or move the binaries back into the gate"
else
    add_pass "R3 registry reviewed ${AGE_DAYS}d ago (within ${STALE_AFTER_DAYS}d)"
fi
echo

# ---- R4: 登记项与代码是否仍一致 ------------------------------------------
echo "--- R4: registered binaries still import no production engine ---"
# 从 runner 取出登记的 binary 名，逐个确认其测试文件里没有 ExecutionEngine。
REGISTERED="$(python3 -c "
import re,sys
src=open(sys.argv[1]).read()
m=re.search(r'OFF_PRODUCTION_PATH_BINARIES\s*=\s*\[(.*?)\]', src, re.S)
if not m:
    print('')
    sys.exit(0)
for name in re.findall(r'\"([^\"]+)\"', m.group(1)):
    print(name)
" "$RUNNER")"

if [ -z "$REGISTERED" ]; then
    add_fail "R4 could not parse OFF_PRODUCTION_PATH_BINARIES from $RUNNER"
else
    echo "  registered: $(echo "$REGISTERED" | tr '\n' ' ')"
    # R4a: 确认排除项确实不在 B2 枚举集合内。
    # B2 枚举 = 根 Cargo.toml 的 [[test]] 名 + EXTRA_TEST_PACKAGES 的 targets
    # （scripts/gate/run_b2_per_binary.py:list_test_binaries）。若某项出现在
    # 枚举里，排除逻辑对它永不触发，清单就成了摆设 —— 此时必须 FAIL。
    ENUM_INFO="$(python3 -c "
import json,re,subprocess
from pathlib import Path
src = open('scripts/gate/run_b2_per_binary.py').read()
extra = re.search(r'EXTRA_TEST_PACKAGES\s*=\s*\[(.*?)\]', src, re.S)
pkgs = re.findall(r'\"([^\"]+)\"', extra.group(1)) if extra else []
names = set(re.findall(r'\[\[test\]\]\s*name\s*=\s*\"([^\"]+)\"', Path('Cargo.toml').read_text()))
try:
    meta = json.loads(subprocess.run(['cargo','metadata','--no-deps','--format-version=1'],
                                     capture_output=True, text=True, timeout=120).stdout)
    for pkg in pkgs:
        names.update(t['name'] for p in meta.get('packages', []) if p['name']==pkg
                     for t in p.get('targets', []) if 'test' in t.get('kind', []))
except Exception:
    pass
print(','.join(sorted(names)))
" 2>/dev/null || echo "")"
    if [ -z "$ENUM_INFO" ]; then
        echo "  [WARN] R4a could not compute the B2 enumeration (cargo metadata?) — skipped"
    else
        while read -r BIN_NAME; do
            [ -z "$BIN_NAME" ] && continue
            if echo ",$ENUM_INFO," | grep -q ",$BIN_NAME,"; then
                add_fail "R4a $BIN_NAME IS in the B2 enumeration — the off-production-path \
exclusion would never trigger. Either remove it from the enumeration \
(EXTRA_TEST_PACKAGES) or re-evaluate: if it now runs the real engine, move it out \
of this registry"
            else
                add_pass "R4a $BIN_NAME is outside the B2 enumeration, as intended"
            fi
        done <<< "$REGISTERED"
    fi

    # R4b: 确认登记项仍不引用生产执行引擎（代码演进会使清单失效）。
    while read -r BIN_NAME; do
        [ -z "$BIN_NAME" ] && continue
        FOUND=""
        for CANDIDATE in crates/planner/tests/"$BIN_NAME".rs \
                         crates/executor/tests/"$BIN_NAME".rs; do
            if [ -f "$CANDIDATE" ]; then
                if grep -qE 'ExecutionEngine|sqlrustgo::' "$CANDIDATE"; then
                    FOUND="$CANDIDATE"
                    break
                fi
            fi
        done
        if [ -n "$FOUND" ]; then
            add_fail "R4b $BIN_NAME now references the production engine ($FOUND) — \
if it became a production-path test, remove it from OFF_PRODUCTION_PATH_BINARIES"
        else
            add_pass "R4b $BIN_NAME still free of production-engine references"
        fi
    done <<< "$REGISTERED"
fi
echo

# ---- R5: #5193 路线状态 --------------------------------------------------
echo "--- R5: #5193 still open (option C still the standing decision) ---"
if curl -sf -u "${GITEA_USER:-openclaw}:${GITEA_PASS:-details8848}" \
        "http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/issues/5193" \
        -o /tmp/_issue5193.json 2>/dev/null; then
    STATE="$(python3 -c "
import json
print(json.load(open('/tmp/_issue5193.json')).get('state','?'))
" 2>/dev/null || echo '?')"
    if [ "$STATE" = "open" ]; then
        add_pass "R5 #5193 still open — option C stands"
    else
        add_fail "R5 #5193 is '$STATE' — if the route changed to A or B, this \
registry must be rewritten (exclusion may no longer apply)"
    fi
elif [ "${IAIA_ALLOW_NETWORK_SKIP:-0}" = "1" ]; then
    # 仅在调用方**显式声明**本次可以跳过网络检查时跳过（例如本地离线开发）。
    #
    # 原本这里是无条件跳过，代价是 R5 可以永久无声失效：CI 上认证信息缺失、
    # Gitea 改名、或网络中断，都会让「选项 C 的前提」检查消失，而门禁仍报
    # PASS —— 一个从不失败的门禁等于没有门禁。
    #
    # CI 不设这个变量，所以那里 R5 要么真的检查、要么 FAIL。
    echo "  [WARN] R5 skipped — IAIA_ALLOW_NETWORK_SKIP=1 was set explicitly"
else
    # 查不到 #5193 的状态，就无法确认选项 C 是否仍然成立。
    # 这是门禁自身的前提，不是可选项：一条无法验证前提的防腐化规则，
    # 会在前提消失后继续显示绿色。
    add_fail "R5 could not read #5193 from the Getea API — the state of the \
option-C decision is unverifiable, so this registry cannot be trusted. \
Set IAIA_ALLOW_NETWORK_SKIP=1 only for genuinely offline local runs."
fi
echo

# ---- 汇总 ----------------------------------------------------------------
echo "=== Result: PASS=${PASS_COUNT} FAIL=${FAIL_COUNT} ==="
if [ "$FAIL_COUNT" -gt 0 ]; then
    echo
    echo "FAILED checks:"
    for item in "${FAIL_ITEMS[@]}"; do echo "  - $item"; done
    echo
    echo "An exclusion whose justification no longer holds is worse than no exclusion:"
    echo "it silently removes coverage while the report still shows a green gate."
    exit 1
fi

echo "All off-production-path exclusions remain justified."
exit 0