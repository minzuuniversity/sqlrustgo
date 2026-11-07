#!/bin/bash
# test_sem1_statement_path.sh — GA-SEM1 判据的**反向变异**夹具。
#
# 2026-11-08（Owner 批准 GA-SEM1 判据修改）：`check_sem1_savepoint.sh` 的第 5 项原来查的是
# `fn execute_savepoint` 这个**实现从未用过**的方法名 ⇒ 恒红的假红。判据改成「SAVEPOINT 语句路径」
# （lexer → parser → savepoint_wiring → engine_dml）后，必须证明：
#
#   A) 真实仓树 ⇒ 判据绿（改完确实能过）
#   B) 抽掉任一接线的变异树 ⇒ 判据红（判据不是恒真、不是空转）
#
# 夹具从门禁源码里**抽取函数本体**再执行（不复制实现 ⇒ 测的就是门禁真正跑的那份逻辑）。
#
# Exit: 0 = 夹具通过（变异被抓住）；1 = 夹具失败（判据无鉴别力 / 真实树被判红）
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
GATE="$SCRIPT_DIR/check_sem1_savepoint.sh"

if [ ! -f "$GATE" ]; then
    echo "FAIL: 找不到门禁脚本 $GATE"
    exit 1
fi

# 抽取 check_savepoint_statement_path 函数（从函数头到行首单独的 }）。
FUNC_BODY="$(awk '/^check_savepoint_statement_path\(\) \{/{f=1} f{print} f&&/^\}/{exit}' "$GATE")"
if [ -z "$FUNC_BODY" ]; then
    echo "FAIL: 门禁里找不到 check_savepoint_statement_path 函数（判据被删/改名 ⇒ 夹具必须红）"
    exit 1
fi
eval "$FUNC_BODY"

WORK="$(mktemp -d "${TMPDIR:-/Volumes/workspace/dev}/sem1fix.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

# --- A) 真实仓树 ⇒ 必须绿 ---
if ! check_savepoint_statement_path "$REPO_ROOT"; then
    echo "FAIL: 真实仓树被判红 ⇒ 判据过严"
    exit 1
fi
echo "[A] OK: 真实仓树判据绿"

# --- B) 变异树：抽掉 engine_dml 里的 record_insert_undo 调用 ⇒ 必须红 ---
FAKE="$WORK/fake"
mkdir -p "$FAKE/crates/parser/src" "$FAKE/src"
cp "$REPO_ROOT/crates/parser/src/lexer.rs" "$FAKE/crates/parser/src/lexer.rs"
cp "$REPO_ROOT/crates/parser/src/parser.rs" "$FAKE/crates/parser/src/parser.rs"
cp "$REPO_ROOT/src/savepoint_wiring.rs" "$FAKE/src/savepoint_wiring.rs"
# engine_dml.rs：把 record_insert_undo( 的调用点删掉（保留文件其余内容）
grep -v "record_insert_undo(" "$REPO_ROOT/src/engine_dml.rs" > "$FAKE/src/engine_dml.rs"
if grep -q "record_insert_undo(" "$FAKE/src/engine_dml.rs"; then
    echo "FAIL: 变异树构造失败（record_insert_undo( 仍在）"
    exit 1
fi

if check_savepoint_statement_path "$FAKE" >/dev/null 2>&1; then
    echo "FAIL: 变异树被判绿 ⇒ 判据无鉴别力（恒真/空转）"
    exit 1
fi
echo "[B] OK: 变异树判红（判据有鉴别力）"

# --- C) 第二处变异：抽掉 parser 的 SavepointStatement 变体 ⇒ 必须红 ---
FAKE2="$WORK/fake2"
mkdir -p "$FAKE2/crates/parser/src" "$FAKE2/src"
cp "$REPO_ROOT/crates/parser/src/lexer.rs" "$FAKE2/crates/parser/src/lexer.rs"
grep -v "Statement::SavepointStatement" "$REPO_ROOT/crates/parser/src/parser.rs" > "$FAKE2/crates/parser/src/parser.rs"
cp "$REPO_ROOT/src/savepoint_wiring.rs" "$FAKE2/src/savepoint_wiring.rs"
cp "$REPO_ROOT/src/engine_dml.rs" "$FAKE2/src/engine_dml.rs"
if check_savepoint_statement_path "$FAKE2" >/dev/null 2>&1; then
    echo "FAIL: 变异树2 被判绿 ⇒ 判据只查了一半路径"
    exit 1
fi
echo "[C] OK: 变异树2 判红"

echo "OK: GA-SEM1 语句路径判据的反向变异成立（A 绿 / B 红 / C 红）"
exit 0
