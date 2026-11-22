#!/bin/bash
# test_criteria_scope_and_types.sh — GA-DRIFT2（legacy 范围）+ `audit` type 白名单的**反向变异**夹具。
#
# 2026-11-08（Owner 批准批次）：
#   * GA-DRIFT2：`check_drift_not_pass.sh` 的扫描范围原先把 `scripts/gate/legacy/`（历史实现、
#     不在 168 分母内、不由任何清单/归档命令集调用）当现行门禁扫 ⇒ 判据陈旧 ⇒ 假红。
#     判据改为**显式排除** legacy ⇒ 必须证明：(A) 排除生效；(B) 现行范围仍有鉴别力；
#     (C) 去掉排除（= 回退成旧行为）⇒ 必须把 legacy 里的反模式重新报出来。
#   * `audit` type：type 表的**唯一权威**是 `docs/governance/CONTRIBUTING.md` §5.2，
#     机器裁决在 `scripts/gate/check_commit_message.sh` 的 ALLOWED_TYPES。
#     两边必须一致（否则 L1 判据自己会红）⇒ 夹具断言「规范表 == ALLOWED_TYPES」。
#
# Exit: 0 = 通过；1 = 失败
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DRIFT_GATE="$REPO_ROOT/scripts/gate/check_drift_not_pass.sh"
CM_GATE="$REPO_ROOT/scripts/gate/check_commit_message.sh"
SPEC="$REPO_ROOT/docs/governance/CONTRIBUTING.md"

for f in "$DRIFT_GATE" "$CM_GATE" "$SPEC"; do
    if [ ! -f "$f" ]; then
        echo "FAIL: 缺少文件 $f"
        exit 1
    fi
done

WORK="$(mktemp -d "${TMPDIR:-/Volumes/workspace/dev}/critfix.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

# ---------------------------------------------------------------------------
# A/B/C) GA-DRIFT2：legacy 排除
# ---------------------------------------------------------------------------
build_scratch_tree() {
    local dir="$1"
    mkdir -p "$dir/gate/legacy" "$dir/tests/baseline"
    cp "$DRIFT_GATE" "$dir/gate/check_drift_not_pass.sh"
    # 反模式样本用**变量拼装**写出来：夹具源码本身不能出现该字面量，
    # 否则 `check_drift_not_pass.sh` 会把**夹具自己**报成反模式（实测踩过）。
    local swallow='|| tr'"ue"
    # 现行门禁里的反模式（必须被报出来 ⇒ 扫描仍有鉴别力）
    cat > "$dir/gate/sample_gate.sh" <<EOF
#!/bin/bash
cargo test --all --no-fail-fast $swallow
EOF
    # legacy 里的同一反模式（排除生效时不得被报出来）
    cat > "$dir/gate/legacy/old_gate.sh" <<EOF
#!/bin/bash
cargo test --all --no-fail-fast $swallow
EOF
}

build_scratch_tree "$WORK/new"
NEW_OUT="$(cd "$WORK/new" && bash gate/check_drift_not_pass.sh 2>&1)"
NEW_RC=$?

if printf '%s\n' "$NEW_OUT" | grep -q "legacy/old_gate.sh"; then
    echo "FAIL: legacy/old_gate.sh 仍被报出来 ⇒ legacy 排除未生效"
    exit 1
fi
echo "[A] OK: legacy/ 的历史实现不被现行判据扫描（排除生效）"

if ! printf '%s\n' "$NEW_OUT" | grep -q "sample_gate.sh"; then
    echo "FAIL: 现行范围内的反模式没被报出来 ⇒ 排除把判据掏空了"
    exit 1
fi
echo "[B] OK: 现行范围内的反模式仍被报出（判据有鉴别力）"

# C) 反向变异：把三处 legacy 排除全部去掉（= 旧行为）⇒ legacy 必须重新被报出来
build_scratch_tree "$WORK/old"
python3 - "$WORK/old/gate/check_drift_not_pass.sh" <<'PY'
import sys
p = sys.argv[1]
s = open(p, encoding="utf-8").read()
before = s
s = s.replace(' ! -path "$SCRIPT_DIR/legacy/*"', '')
s = s.replace(" --exclude-dir='legacy'", '')
if s == before:
    print("MUTATE_FAIL: 没找到 legacy 排除（判据已不是预期形态）")
    sys.exit(3)
open(p, "w", encoding="utf-8").write(s)
PY
if [ $? -ne 0 ]; then
    echo "FAIL: 反向变异构造失败"
    exit 1
fi
OLD_OUT="$(cd "$WORK/old" && bash gate/check_drift_not_pass.sh 2>&1)"
OLD_RC=$?
if ! printf '%s\n' "$OLD_OUT" | grep -q "legacy/old_gate.sh"; then
    echo "FAIL: 去掉 legacy 排除后 legacy 反模式仍未被报出 ⇒ 夹具对本次修改无鉴别力"
    exit 1
fi
echo "[C] OK: 去掉排除（回退成旧行为）⇒ legacy 反模式重新被报出"

# ---------------------------------------------------------------------------
# D) `audit` type：规范表 == ALLOWED_TYPES（双向一致）
# ---------------------------------------------------------------------------
spec_types() {
    awk -F'|' '/^\| `[a-z]+` \|/ {gsub(/[` ]/, "", $2); print $2}' "$SPEC" | sort -u
}
gate_types() {
    awk -F'"' '/^ALLOWED_TYPES=/ {print $2}' "$CM_GATE" | tr ' ' '\n' | sort -u
}
SPEC_TYPES="$(spec_types)"
GATE_TYPES="$(gate_types)"
if [ "$SPEC_TYPES" != "$GATE_TYPES" ]; then
    echo "FAIL: type 表与机器判据不一致"
    echo "  规范表: $(printf '%s' "$SPEC_TYPES" | tr '\n' ' ')"
    echo "  门禁  : $(printf '%s' "$GATE_TYPES" | tr '\n' ' ')"
    exit 1
fi
if ! printf '%s\n' "$GATE_TYPES" | grep -qx "audit"; then
    echo "FAIL: `audit` 不在 type 白名单里"
    exit 1
fi
echo "[D] OK: 规范表 == ALLOWED_TYPES（$(printf '%s' "$GATE_TYPES" | tr '\n' ' ')）"

# E) 反向变异：从门禁白名单里去掉 audit ⇒ 一致性判据必须红
python3 - "$CM_GATE" "$WORK/cm_mutated.sh" <<'PY'
import sys
src, dst = sys.argv[1], sys.argv[2]
s = open(src, encoding="utf-8").read()
out = s.replace('ALLOWED_TYPES="feat fix refactor docs test chore perf ci style merge audit evidence bench spec"',
                'ALLOWED_TYPES="feat fix refactor docs test chore perf ci style merge evidence bench spec"')
if out == s:
    print("MUTATE_FAIL: 没找到 ALLOWED_TYPES 行（形态与预期不符）")
    sys.exit(3)
open(dst, "w", encoding="utf-8").write(out)
PY
if [ $? -ne 0 ]; then
    echo "FAIL: 反向变异构造失败"
    exit 1
fi
MUT_TYPES="$(awk -F'"' '/^ALLOWED_TYPES=/ {print $2}' "$WORK/cm_mutated.sh" | tr ' ' '\n' | sort -u)"
if [ "$SPEC_TYPES" = "$MUT_TYPES" ]; then
    echo "FAIL: 去掉 audit 后仍被判一致 ⇒ 夹具无鉴别力"
    exit 1
fi
echo "[E] OK: 去掉 audit ⇒ 一致性判据判红"

echo "OK: GA-DRIFT2 legacy 排除 + audit type 白名单的反向变异成立"
exit 0
