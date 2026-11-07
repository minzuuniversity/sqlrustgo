#!/bin/bash
# test_rc_ga_gate_verdict.sh — GA-DRIFT1 判据的**反向变异**夹具。
#
# 2026-11-08（Owner 批准 GA-DRIFT1 判据修改）：`check_rc_ga_gate.sh` 有两处判据缺陷：
#   (1) DRIFT 分支的文案写「tracked, not blocking」，而 `gate_verdict_for` 里 DRIFT 就是
#       GATE_EXIT_CODE=2 ⇒ **本来就阻断** ⇒ 文案与真实判定相反（误导）。
#   (2) FAIL 分支把 D1/D2/D8/D9 的 blocker 数写死成一句「D1/D2/D8/D9 blockers>0」，
#       不打印真实计数 ⇒ 红了也看不出是哪一维。
#
# 夹具证明：改完 ⇒ 文案如实 + 逐维实测值；**改回去（变异）⇒ 夹具必须红**。
#
# Exit: 0 = 通过；1 = 失败（判据被回退 / 无鉴别力）
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GATE="$SCRIPT_DIR/check_rc_ga_gate.sh"

if [ ! -f "$GATE" ]; then
    echo "FAIL: 找不到门禁脚本 $GATE"
    exit 1
fi

# --- 1) 行为断言：DRIFT 判定本身必须是「阻断」（非 0 退出码） ---
# 只抽取「无副作用的纯函数」本体来执行（不 source 整个门禁：门禁顶层有依赖外部环境的代码，
# 在 set -u 下会直接终止夹具）。抽取而不是复制 ⇒ 测的就是门禁真正跑的那份逻辑。
for fn in gate_dimensions_for_mode gate_expected_dims gate_verdict_for; do
    body="$(awk -v fn="$fn" '$0 ~ "^" fn "\\(\\) \\{" {f=1} f{print} f&&/^\}/{exit}' "$GATE")"
    if [ -z "$body" ]; then
        echo "FAIL: 门禁里找不到函数 $fn（判据被删/改名 ⇒ 夹具必须红）"
        exit 1
    fi
    eval "$body"
done

# 注意：变量后紧跟全角标点时**必须**写成 ${VAR}，否则 bash 会把「全角括号」并进变量名
#（在 set -u 下直接报 unbound variable —— 已实测踩过）。
gate_verdict_for "all" "D1 D2 D3 D4 D5 D6 D7 D8 D9" 0 0 0 0 0 0 0 1 0
if [ "${GATE_VERDICT}" != "DRIFT" ]; then
    echo "FAIL: d3d>0 时判定不是 DRIFT（实际 ${GATE_VERDICT}）"
    exit 1
fi
if [ "${GATE_EXIT_CODE:-0}" -eq 0 ]; then
    echo "FAIL: DRIFT 的退出码是 0 ⇒ 不阻断（与「drift 不算放行」的口径矛盾）"
    exit 1
fi
echo "[1] OK: DRIFT ⇒ VERDICT=${GATE_VERDICT} / EXIT_CODE=${GATE_EXIT_CODE}（阻断）"

# --- 2) 文案断言（对给定文件求值，便于做反向变异） ---
# 关键区分（两个不同的概念，不能混为一谈）：
#   * **判定级 DRIFT**（gate_verdict_for ⇒ GATE_VERDICT=DRIFT ⇒ exit 2）= 阻断；
#     它的输出行必须如实写「阻断」。
#   * **维度级漂移**（C-ARCH-05 的已登记漂移、D8 的维度计数）= STAGE allowlist 认可的
#     「记录不阻断」，它自己的措辞是合法的，**不属于**本判据范围。
# 所以断言只锁定「判定级 DRIFT 分支」这个块，不对全文件做字面匹配。
# 先剥掉行尾注释再匹配（与仓库其它门禁同一做法）：注释里引用旧文案是说明，不是判定输出。
extract_verdict_drift_block() {
    awk '
        /elif \[\[ "\$VERDICT" == "DRIFT" \]\]/ {f=1}
        f && /^[[:space:]]*else[[:space:]]*$/ {exit}
        f {print}
    ' "$1" | sed 's/[[:space:]]*#.*$//'
}

check_drift_text() {
    local f="$1"
    local block
    block="$(extract_verdict_drift_block "$f")"
    if [ -z "$block" ]; then
        echo "FAIL: 找不到判定级 DRIFT 分支（elif [[ \"\$VERDICT\" == \"DRIFT\" ]] …）"
        return 1
    fi
    if printf '%s\n' "$block" | grep -q "not blocking"; then
        echo "FAIL: 判定级 DRIFT 分支仍写着「not blocking」⇒ 与 exit 2 的真实判定矛盾"
        return 1
    fi
    if ! printf '%s\n' "$block" | grep -q "blocking"; then
        echo "FAIL: 判定级 DRIFT 分支没有如实说明「阻断」"
        return 1
    fi
    local stripped
    stripped="$(sed 's/[[:space:]]*#.*$//' "$f")"
    if printf '%s\n' "$stripped" | grep -q "D1/D2/D8/D9 blockers>0"; then
        echo "FAIL: FAIL 分支仍用写死串「D1/D2/D8/D9 blockers>0」⇒ 看不到真实计数"
        return 1
    fi
    if ! printf '%s\n' "$stripped" | grep -q "D1_BLOCKERS="; then
        echo "FAIL: FAIL 分支没有逐维打印实测计数（D1_BLOCKERS=…）"
        return 1
    fi
    return 0
}

if ! check_drift_text "$GATE"; then
    echo "FAIL: 真实门禁未通过文案判据（判据修改未落地？）"
    exit 1
fi
echo "[2] OK: 真实门禁文案如实（DRIFT=阻断 + 逐维实测计数）"

# --- 3) 反向变异：把文案改回旧样 ⇒ 判据必须红 ---
WORK="$(mktemp -d "${TMPDIR:-/Volumes/workspace/dev}/driftfix.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT
MUT="$WORK/check_rc_ga_gate_mutated.sh"
# 构造真正的「旧样」：把新文案整行换回旧文案
python3 - "$GATE" "$MUT" <<'PY'
import sys, re
src, dst = sys.argv[1], sys.argv[2]
text = open(src, encoding="utf-8").read()
old_line = 'echo -e "  ${YELLOW}⚡ GATE: DRIFT${NC} — Drift detected, tracked, not blocking"'
# 把当前的 DRIFT 输出行整行替换成旧文案
new = re.sub(r'echo -e "  \$\{RED\}✗ \[FAIL\] GATE: DRIFT\$\{NC\}[^\n]*\n', old_line + "\n", text, count=1)
if new == text:
    print("MUTATE_FAIL: 没能定位 DRIFT 输出行")
    sys.exit(3)
open(dst, "w", encoding="utf-8").write(new)
PY
if [ $? -ne 0 ]; then
    echo "FAIL: 反向变异构造失败"
    exit 1
fi
if check_drift_text "$MUT" >/dev/null 2>&1; then
    echo "FAIL: 变异样被判绿 ⇒ 文案判据无鉴别力"
    exit 1
fi
echo "[3] OK: 回退成旧文案的变异样判红"

echo "OK: GA-DRIFT1 判据的反向变异成立（真实门禁绿 / 回退样红）"
exit 0
