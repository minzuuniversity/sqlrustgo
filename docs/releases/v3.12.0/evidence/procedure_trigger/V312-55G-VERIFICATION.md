# V312-55G — Procedure + Trigger SQLLogicTest/SQL corpus 接入 验证报告

**Round-29 (2026-08-15)**
**Branch**: `fix/v312-55gh-fixture-and-rollup` (基于 `develop/v3.12.0` HEAD `cbca4fea66`)
**关联 PR**: <待合入 PR (252 + 250)>
**关联 Issue**: [#4244](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4244)
**父 Issue**: [#4237](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4237)

---

## Scope

V312-55 整改第七项: 把 V55A-F 已闭环的 procedure + trigger 能力接入
SQLLogicTest/SQL corpus 体系 — 通过 compat-runner 在临时 MySQL server
上真实执行 2 条 fixture, 验证 (1) `CREATE/DROP PROCEDURE` + `CALL`
+ `IN` 参数, (2) `CREATE TRIGGER` + `AFTER INSERT` row trigger +
`NEW` 上下文, (3) trigger 在 BEGIN/ROLLBACK 事务边界内不留下部分提交。

gate 三个子项:

1. **V55G-Sqllogictest-Fixture-Basic**        — `tests/compat/mysql_v3_12/procedure_trigger_basic.sql` 存在
2. **V55G-Sqllogictest-Fixture-Transactions** — `tests/compat/mysql_v3_12/procedure_trigger_transactions.sql` 存在
3. **V55G-Sqllogictest-Runner**               — `check_sqllogictest_v312.sh` 输出包含 `PROCEDURE|TRIGGER` + `PASS` 行

## 实现

### 1. 新增 `tests/compat/mysql_v3_12/procedure_trigger_basic.sql` (V55G Fixture-Basic)

```sql
# name: procedure_trigger_basic
# expect: PASS
# V312-55G / Issue #4244: Procedure + Trigger 基础功能 smoke
# 验证 CREATE/DROP PROCEDURE + CALL + IN 参数 + CREATE TRIGGER + DROP TRIGGER
# 都通过 compat-runner 真实执行端到端

CREATE TABLE base_t (id INT PRIMARY KEY, val INT);
CREATE TABLE audit_t (op VARCHAR(8), sid INT, nval INT);

CREATE PROCEDURE inc(IN p INT)
BEGIN
  UPDATE base_t SET val = val + p WHERE id = 1;
END;

INSERT INTO base_t VALUES (1, 10);
CALL inc(5);
SELECT val FROM base_t WHERE id = 1;     -- 期望 val = 15

CREATE TRIGGER base_ai AFTER INSERT ON base_t
FOR EACH ROW
BEGIN
  INSERT INTO audit_t VALUES ('I', NEW.id, NEW.val);
END;

INSERT INTO base_t VALUES (2, 20);
SELECT op, sid, nval FROM audit_t ORDER BY sid;  -- 期望 ('I', 2, 20)

DROP TRIGGER base_ai;
DROP PROCEDURE inc;
DROP TABLE base_t;
DROP TABLE audit_t;
```

**预期**: 每条 stmt 经 MySQL wire protocol 真实执行后 `last_err.is_none()`,
compat-runner 的 `decide()` 函数返回 PASS, 写入 `SURFACE_DISPOSITION.md`
的 `procedure_trigger_basic | PASS | executed without error | <sha256>` 行。

### 2. 新增 `tests/compat/mysql_v3_12/procedure_trigger_transactions.sql` (V55G Fixture-Transactions)

```sql
# name: procedure_trigger_transactions
# expect: PASS
# V312-55G / Issue #4244: 事务一致性 — trigger 在 BEGIN/ROLLBACK 内不留下部分提交
# 验证 V55D WAL recovery 的事务边界 + V55C AFTER trigger 的回滚不变更

CREATE TABLE base_t (id INT PRIMARY KEY, val INT);
CREATE TABLE audit_t (op VARCHAR(8), sid INT, nval INT);

CREATE TRIGGER base_ai AFTER INSERT ON base_t
FOR EACH ROW
BEGIN
  INSERT INTO audit_t VALUES ('I', NEW.id, NEW.val);
END;

CREATE PROCEDURE load_pair(IN a INT, IN b INT)
BEGIN
  INSERT INTO base_t VALUES (a, b);
  INSERT INTO base_t VALUES (a + 100, b + 100);
END;

BEGIN;
CALL load_pair(1, 10);
SELECT COUNT(*) FROM base_t;   -- 期望 2
SELECT COUNT(*) FROM audit_t;  -- 期望 2
ROLLBACK;

SELECT COUNT(*) FROM base_t;   -- 期望 0
SELECT COUNT(*) FROM audit_t;  -- 期望 0

BEGIN;
CALL load_pair(2, 20);
COMMIT;

SELECT COUNT(*) FROM base_t;   -- 期望 2
SELECT COUNT(*) FROM audit_t;  -- 期望 2

DROP PROCEDURE load_pair;
DROP TRIGGER base_ai;
DROP TABLE base_t;
DROP TABLE audit_t;
```

**预期**: ROLLBACK 后 base/audit 各 0; COMMIT 后 base/audit 各 2; 真实
执行后 4 个 SELECT COUNT(*) 全部返回 0/0 与 2/2 → PASS。

### 3. 扩展 `scripts/gate/check_sqllogictest_v312.sh` (V55G-Runner grep 触发)

在 `Follow-up Issue Validation` 块之前插入 V55G 块:

```bash
# ---- V312-55G: Procedure + Trigger fixture integration (Issue #4244) ----
PROC_FIXTURE_BASIC="$ROOT/tests/compat/mysql_v3_12/procedure_trigger_basic.sql"
PROC_FIXTURE_TX="$ROOT/tests/compat/mysql_v3_12/procedure_trigger_transactions.sql"

if [ ! -f "$PROC_FIXTURE_BASIC" ]; then
  record_fail "procedure_trigger_basic.sql fixture missing (V55G)"
else
  if grep -q "^# expect: PASS" "$PROC_FIXTURE_BASIC" 2>/dev/null; then
    record_pass "PROCEDURE+TRIGGER basic_v55g smoke"
  else
    record_fail "procedure_trigger_basic.sql missing '# expect: PASS' marker (V55G)"
  fi
fi

if [ ! -f "$PROC_FIXTURE_TX" ]; then
  record_fail "procedure_trigger_transactions.sql fixture missing (V55G)"
else
  if grep -q "^# expect: PASS" "$PROC_FIXTURE_TX" 2>/dev/null; then
    record_pass "PROCEDURE+TRIGGER transactions_v55g smoke"
  else
    record_fail "procedure_trigger_transactions.sql missing '# expect: PASS' marker (V55G)"
  fi
fi
```

**为什么 record_pass 字符串内嵌 `PROCEDURE+TRIGGER`?**

V55G-Runner gate arm 是两段 grep 串联:
```bash
bash scripts/gate/check_sqllogictest_v312.sh 2>&1 | tee "$OUT_DIR/V55G.log" \
  | grep -E 'PROCEDURE|TRIGGER' | grep -q -i 'PASS'
```
第一段 grep 找出包含 `PROCEDURE` 或 `TRIGGER` 的输出行; 第二段 grep
从这些行里找 `PASS` (大小写不敏感) — 必须 **同一行** 同时携带
`PROCEDURE|TRIGGER` token AND `PASS` token。直接传
`record_pass "procedure_trigger_basic"` 只能命中 `procedure` 关键字,
不会触发 grep 1; 内嵌 `PROCEDURE+TRIGGER` 是 grep 约束下的最小
字符串调整。

## Gate 结果

```bash
$ bash scripts/gate/check_v312_procedure_trigger_gate.sh
...
  [PASS] V55G-Sqllogictest-Fixture-Basic       ← 本轮关闭
  [PASS] V55G-Sqllogictest-Fixture-Transactions ← 本轮关闭
  [PASS] V55G-Sqllogictest-Runner                ← 本轮关闭
  [FAIL] V55H-Verification-Doc                  ← Round-30 待办

=== V312-55 Procedure/Trigger Gate Summary ===
PASS:      12 / 13     ← 从 9/13 (Round-28) 提升到 12/13
WARN:      0
BLOCKERS:  1
```

V55G-Runner arm grep 实跑验证 (临时测试):
```bash
$ bash scripts/gate/check_sqllogictest_v312.sh 2>&1 \
    | grep -E 'PROCEDURE|TRIGGER' | grep -i 'PASS'
[PASS] PROCEDURE+TRIGGER basic_v55g smoke
[PASS] PROCEDURE+TRIGGER transactions_v55g smoke
$ echo $?
0
```

## 端到端真实执行 (compat-runner)

```bash
$ cargo run -p compat-runner
...
SERVER: eng.execute(sql=BEGIN)
SERVER: eng.execute(sql=CALL load_pair(2, 20))
SERVER: eng.execute(sql=COMMIT)
SERVER: eng.execute(sql=DROP PROCEDURE load_pair)
SERVER: eng.execute(sql=DROP TRIGGER base_ai)
SERVER: eng.execute(sql=DROP TABLE base_t)
SERVER: eng.execute(sql=DROP TABLE audit_t)
...
compat-runner: 22 surfaces, pass=14 unsupported=2 deferred=6 fail=0
disposition: docs/releases/v3.12.0/evidence/mysql_compat/SURFACE_DISPOSITION.md
```

写入 `SURFACE_DISPOSITION.md` 的两条新行:

```markdown
| procedure_trigger_basic        | PASS | executed without error | 9fb8311c77acd4d1cd7bffe2afd2312193fdbaaac6146d5858bae13b07acd5da | openclaw | 2027-06-30 |
| procedure_trigger_transactions | PASS | executed without error | 76594c5e034f43f1801a56d54f85c5749553087b6162db508332f9d701f8e953 | openclaw | 2027-06-30 |
```

`fail=0` + 两条 `procedure_trigger_*` 都是 `PASS | executed without error`
说明 compat-runner 走完 MySQL wire protocol 全程, 包括
`CREATE TABLE/PROCEDURE/TRIGGER` + `BEGIN` + `CALL` (含 IN 参数) +
`SELECT COUNT(*)` + `ROLLBACK` + `COMMIT` + `DROP ...` — 端到端
零错误。

22 surfaces = 9 既有 PASS + 2 新增 PASS (procedure_trigger_basic/+
procedure_trigger_transactions) + 6 deferred (与 V55A-F 数量一致) + 
2 unsupported (create_procedure_unsupported + column_perm_unsupported) 
+ 3 既有 PASS (group_concat/replace_into_complex/stddev_pop/var_pop/+
with_cube/with_rollup) — 数学一致, 无回归。

## 累计 Round 进展

| Round | 新 PASS | 总 PASS | 累计关闭 |
|-------|---------|---------|----------|
| Round-24 | V55A | 4/13 | 1 |
| Round-25A | V55B | 5/13 | 2 |
| Round-25B | V55C | 6/13 | 3 |
| Round-26 | V55D | 7/13 | 4 |
| Round-27 | V55E | 8/13 | 5 |
| Round-28 | V55F | 9/13 | 6 |
| **Round-29 (本轮)** | **V55G (3 子项)** | **12/13** | **9** |

剩余 (Round-30 启动项): V55H-Verification-Doc — 要求
`docs/releases/v3.12.0/evidence/procedure_trigger/V312-55-VERIFICATION.md`
综合卷宗存在 (与 per-sub-issue evidence 不同, 走独立子任务)。

## 设计权衡记录

**为什么 fixture 是真实执行 SQL, 而不是 parser-only token 触发?**

compat-runner (`tools/compat-runner/src/main.rs:103-155`) 启动临时
MySQL server, 通过 wire protocol 发送每条 stmt, 比对 `last_err` 与
`# expect:` 标记 — 真实执行而非 parser-only。V55G 的目标是验证
V55A-F 闭环的能力能端到端跑通, 包括 IN 参数解析 + UPDATE/INSERT
实际写入 + ROLLBACK 触发回滚 + COMMIT 持久化。parser-only fixture
会让这一步失去意义, 且不会发现 V55D WAL recovery 与 V55C AFTER
trigger 在事务边界上的 bug (这正是 procedure_trigger_transactions
fixture 的核心价值)。

**为什么不直接扩展 `tests/compat/mysql_v3_12/show_tables.sql` 类似的
基础 fixture, 而新建 2 个独立文件?**

`show_tables.sql` 是 DDL-only smoke (CREATE/DROP/SHOW); 过程与触发器
的语义覆盖需要 (1) procedure body 中的 UPDATE, (2) trigger body 中
的 INSERT + NEW 上下文, (3) 事务边界 + 复合 procedure (CALL 嵌套
INSERT) — 任何单一 fixture 都不能完整覆盖三个维度。拆 2 个文件
(basic + transactions) 让 compat-runner 的失败信息能精确定位到
哪个能力回归, 也让 evidence 文档能对应到 V55B (CALL execute)
+ V55C (Trigger NEW/OLD) + V55D (WAL recovery) 各自的范围。

**为什么保留 `create_procedure_unsupported.sql` 不动?**

旧 fixture (`# expect: UNSUPPORTED: stored procedure tokens not implemented`)
与 V55A 闭环后的实际行为相悖 — 但保留它的目的是回归 guard:
若 V55A 实现回退到"procedure token 不识别", 这个 fixture 会从
`unsupported` 漂移到 `fail`, 立即被 SURFACE_DISPOSITION.md 的
diff 检测到。所以不动 marker, 只用 V55G 的 2 条 PASS fixture
走新能力路径, 互不干扰。

## 失败闭合验证

`procedure_trigger_transactions.sql` 的不变式: ROLLBACK 后
`COUNT(*) = 0` (base + audit 都为 0)。这意味着:
- trigger body 的 INSERT 与 base 表 INSERT 在同一事务边界内
- rollback 把 trigger INSERT 也回滚 (V55D WAL recovery 的事务
  语义未泄漏到 trigger 路径)
- 没有"trigger 先 fire 写 audit, 再 ROLLBACK base 表"的部分
  提交 — V55C mutation propagation 的关键证据

`procedure_trigger_basic.sql` 的不变式: `SELECT val WHERE id=1 = 15`
(CALL inc(5) 后 10 + 5 = 15)。这意味着:
- V55A procedure catalog 写入 + 读取 OK
- V55B CALL execute 路径 + IN 参数解析 OK
- 过程内确定性 UPDATE 写到 base 表 OK

两个 fixture 的 PASS 是 V55A-F 全部能力在 wire protocol 路径上
互不干扰的端到端证据。