#!/bin/bash
# test_idx1_mutation_fixtures.sh — 看板行 IDX-1 判据的**反向变异**夹具。
#
# 证两件事：
#   A) 真实树上 IDX-1 的行为判据全绿；
#   B) 把修好的**取库点**逐个改回改动前的写法 ⇒ 对应判据必须红。
# 没有 B，「判据覆盖了缺陷」就只是文字主张（IDX-1 验收第 ③ 条明确要求反向变异证据）。
#
# 四处变异（各自单独做，做完立刻恢复）：
#   M1 会话库解析 → 只看默认库（`execute_create_index`，`src/execution_engine_methods.rs`）
#      ⇒ 读数 ①②④ 红：`USE d1` 之后建索引又变成 `Table not found: t`
#   M2 索引落盘按库 → `save_index`（硬编码 DEFAULT_DATABASE，`file_storage.rs`）
#      ⇒ 读数 ① 的落盘断言红：d1 的索引被写进默认库目录
#   M3 `INDEXED BY` 的库过滤去掉 → 按索引名全局认领（`src/engine_select.rs`）
#      ⇒ 读数 ④ 红：跨库同名索引互相顶掉 / 别的库也能认领别人的索引
#   M4 PK 重建的取键回到 `self.tbl(&scoped_table)`（在已加域的键上再加一层）
#      ⇒ PK 两条判据红：重启后 PK 索引查不到；d1 的索引文件不存在
#
# 每处 sed 都必须**真的改动文件**（`cmp` 校验）：sed 空转会让夹具假装通过而毫无鉴别力。
#
# Exit: 0 = 判据有鉴别力（A 全绿 + 四个变异各自变红）；1 = 判据失效或变异没生效
set -u

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT" || exit 1
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"

EEM="src/execution_engine_methods.rs"
FS="crates/storage/src/file_storage.rs"
SEL="src/engine_select.rs"
SRC_LIST="$EEM
$FS
$SEL"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

key_of() { echo "$1" | tr '/' '_'; }

for f in $SRC_LIST; do
    if [ ! -f "$f" ]; then
        echo "FAIL: 找不到待变异文件 $f（文件被移动/改名 ⇒ 夹具必须红，否则变异目标已失效）"
        exit 1
    fi
    cp "$f" "$WORK/$(key_of "$f").orig"
done

restore_all() {
    for f in $SRC_LIST; do
        cp "$WORK/$(key_of "$f").orig" "$f"
    done
}
trap 'restore_all; rm -rf "$WORK"' EXIT

# mutate <file> <sed 程序>：改动必须生效，否则直接判红（sed 空转 = 假证据）
mutate() {
    local file="$1" prog="$2"
    # BSD/GNU 都可用的原地写法（macOS 的 `sed -i` 必须带扩展名参数，否则把程序当后缀）
    sed -E -i.bak "$prog" "$file"
    rm -f "$file.bak"
    if cmp -s "$WORK/$(key_of "$file").orig" "$file"; then
        echo "FAIL: 变异没有改动 $file（sed 模式失配 ⇒ 夹具失去鉴别力）"
        echo "      模式: $prog"
        exit 1
    fi
}

idx1_run() { cargo test --test idx1_session_db_index_test 2>&1; }
pk_run() { cargo test -p sqlrustgo-storage --test pk_rebuild_scoped_5592 2>&1; }

# 期望：目标必须**失败**，且失败的是指定那条测试。
# 编译失败不算「判据变红」（那说明变异不可编译，不是判据起了作用）。
assert_red_with() {
    out="$1"
    name="$2"
    label="$3"
    if ! echo "$out" | grep -q "^test result: FAILED"; then
        if echo "$out" | grep -q "^error\[E"; then
            echo "FAIL: $label 的变异导致编译错误，判据没被跑到："
            echo "$out" | grep "^error" | head -5
        else
            echo "FAIL: $label 变异后测试仍然全绿 ⇒ 该判据不覆盖这个缺陷"
        fi
        exit 1
    fi
    if ! echo "$out" | grep -q "^test $name \.\.\. FAILED"; then
        echo "FAIL: $label 变异后 $name 没有变红（判据不针对该缺陷）"
        echo "$out" | grep -E "^test .* \.\.\. (ok|FAILED)" | head -8
        exit 1
    fi
}

assert_green() {
    out="$1"
    label="$2"
    if ! echo "$out" | grep -q "^test result: ok"; then
        echo "FAIL: $label 在真实树上不绿 ⇒ 夹具无法证明「改完转绿」"
        echo "$out" | tail -20
        exit 1
    fi
    echo "  $label: $(echo "$out" | grep '^test result' | head -1)"
}

echo "== A) 真实树基线（必须全绿）=="
assert_green "$(idx1_run)" "IDX-1 行为判据"
assert_green "$(pk_run)" "PK 重建判据"

echo "== M1) 会话库解析 → 只看默认库 =="
mutate "$EEM" 's/\.unwrap_or\(session_db\.as_str\(\)\)/.unwrap_or("default")/'
assert_red_with "$(idx1_run)" "test_idx1_create_index_follows_session_database" "M1"
restore_all

echo "== M2) 索引落盘按库 → save_index（默认库）=="
mutate "$FS" 's/self\.save_index_in\(db, table, &column_name, &index\)/self.save_index(table, \&column_name, \&index)/'
assert_red_with "$(idx1_run)" "test_idx1_create_index_follows_session_database" "M2"
restore_all

echo "== M3) INDEXED BY 去掉库过滤 → 按名字全局认领 =="
mutate "$SEL" 's/\.find\(\|i\| i\.name == \*idx_name \&\& i\.database\.eq_ignore_ascii_case\(&base_db\)\);/.find(|i| i.name == *idx_name);/'
assert_red_with "$(idx1_run)" "test_idx1_index_is_scoped_per_database" "M3"
restore_all

echo "== M4) PK 重建取键回到 self.tbl（在已加域的键上再加一层）=="
mutate "$FS" 's/s\.tables\.get\(&scoped_table\)/s.tables.get(\&self.tbl(\&scoped_table))/'
assert_red_with "$(pk_run)" "pk_index_is_rebuilt_and_searchable" "M4"
restore_all

echo "== E) 恢复后复跑（确认恢复干净）=="
assert_green "$(idx1_run)" "IDX-1 行为判据（恢复后）"
assert_green "$(pk_run)" "PK 重建判据（恢复后）"

for f in $SRC_LIST; do
    if ! cmp -s "$WORK/$(key_of "$f").orig" "$f"; then
        echo "FAIL: 恢复后 $f 仍与原始内容不一致"
        exit 1
    fi
done

echo "OK: IDX-1 判据有鉴别力（基线全绿；四处取库点各自回退都能打红对应判据）"
exit 0
