# V312-55 — 存储过程和触发器整改总体 verification 报告

> **Issue:** [#4237](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4237) (master) + [#4244](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4244) (V55G) + [#4245](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4245) (V55H)
> **provenance:** generated_by=openclaw-minimax, generated_at=2026-08-15T02:32:00Z, commit=cbca4fea66 (pre-merge HEAD, V55G/H work landed on top via branch fix/v312-55gh-fixture-and-rollup), branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0
> **purpose:** V312-55 八子项总卷宗 + V55H-Evidence roll-up 关闭证据

---

## 1. 范围

### 1.1 已闭环能力 (v3.12.0)

**存储过程** (V55A + V55B + V55F):
- `CREATE PROCEDURE` / `DROP PROCEDURE` / `SHOW PROCEDURE` 字典操作
- `CALL p(args)` 执行 + `IN` 参数解析
- 过程内确定性 SQL (INSERT / UPDATE / DELETE / SELECT)
- 非 root 用户 5 路径权限模型 (CREATE/DROP/CALL + procedure namespace)

**触发器** (V55C + V55D + V55E + V55F):
- `BEFORE/AFTER INSERT/UPDATE/DELETE` row trigger
- `NEW` / `OLD` 上下文引用
- trigger body DML 在事务边界内回滚 (WAL recovery 一致)
- trigger 递归深度限制 (MAX_RECURSION_DEPTH = 16, RAII DepthGuard)
- 非 root 用户 5 路径权限模型 (CREATE TRIGGER + trigger body DML hook)

**SQL corpus 接入** (V55G):
- 2 个 compat-runner fixture (`procedure_trigger_basic.sql` + `procedure_trigger_transactions.sql`)
- 端到端通过 MySQL wire protocol 真实执行, 无回归
- 22 surfaces = 14 PASS + 6 deferred + 2 unsupported + 0 fail

### 1.2 显式延后 (v3.13+)

- `OUT` / `INOUT` 参数
- `DEFINER` / `SQL SECURITY` 子句
- 动态 SQL inside procedure (PREPARE/EXECUTE inside CALL)
- cursor / handler / condition (完整 PL/SQL)
- trigger `FOR EACH STATEMENT` (目前只支持 row)

### 1.3 OUT OF SCOPE

- 完整 PL/SQL (DECLARE ... BEGIN ... EXCEPTION ... END)
- oracle-compatible 存储过程语法
- 触发器的 `BEFORE` row trigger 中的 mutation (V55C 已闭环)

## 2. 子项完成情况

| 子项 | Issue | PR | Evidence | Gate arm | Round |
|---|---|---|---|---|---|
| V55A Procedure DDL | [#4238](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4238) | [#4259](http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/4259) | [V312-55A-VERIFICATION.md](V312-55A-VERIFICATION.md) | PASS | Round-24 |
| V55B CALL execute | [#4239](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4239) | [#4262](http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/4262) | [V312-55B-VERIFICATION.md](V312-55B-VERIFICATION.md) | PASS | Round-25A |
| V55C Trigger NEW/OLD | [#4240](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4240) | [#4262](http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/4262) | [V312-55C-VERIFICATION.md](V312-55C-VERIFICATION.md) | PASS | Round-25B |
| V55D WAL recovery | [#4241](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4241) | [#4262](http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/4262) | [V312-55D-VERIFICATION.md](V312-55D-VERIFICATION.md) | PASS | Round-26 |
| V55E Recursion limit | [#4242](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4242) | [#4264](http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/4264) | [V312-55E-VERIFICATION.md](V312-55E-VERIFICATION.md) | PASS | Round-27 |
| V55F Privilege model | [#4243](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4243) | [#4264](http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/4264) | [V312-55F-VERIFICATION.md](V312-55F-VERIFICATION.md) | PASS | Round-28 |
| V55G SQL corpus | [#4244](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4244) | <本 PR> | [V312-55G-VERIFICATION.md](V312-55G-VERIFICATION.md) | PASS | Round-29 |
| V55H Evidence roll-up | [#4245](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4245) | <本 PR> | V312-55-VERIFICATION.md (本文件) | PASS | Round-30 |

## 3. Gate 关闭命令原样

```bash
# 1. 语法检查
bash -n scripts/gate/check_v312_procedure_trigger_gate.sh

# 2. 完整 gate (期望 13/13 PASS, exit 0)
bash scripts/gate/check_v312_procedure_trigger_gate.sh

# 3. JSON 状态 (期望 status=PASS)
bash scripts/gate/check_v312_procedure_trigger_gate.sh --json

# 4. 关联 evidence 路径
ls -la docs/releases/v3.12.0/evidence/procedure_trigger/

# 5. compat-runner 端到端 (期望 procedure_trigger_* 两条 PASS)
cargo build -p compat-runner && cargo run -p compat-runner
grep -E 'procedure_trigger' docs/releases/v3.12.0/evidence/mysql_compat/SURFACE_DISPOSITION.md
```

## 4. 实跑退出码 + 关键输出 (Round-30 收尾时)

```
$ bash scripts/gate/check_v312_procedure_trigger_gate.sh
=== V312-55 Procedure/Trigger Gate (v3.12.0) ===
  [PASS] GATE-Script-Syntax
  [PASS] V55-Plan-Doc-Exists
  [PASS] V55A-Procedure-DDL
  [PASS] V55B-Call-Execute
  [PASS] V55C-Trigger-NewOld
  [PASS] V55D-WAL-Recovery
  [PASS] V55E-Recursion
  [PASS] V55F-Privilege
  [PASS] V55G-Sqllogictest-Fixture-Basic
  [PASS] V55G-Sqllogictest-Fixture-Transactions
  [PASS] V55G-Sqllogictest-Runner
  [PASS] V55H-Verification-Doc
  [PASS] ANTI-Ignore-Procedure-Tests

=== V312-55 Procedure/Trigger Gate Summary ===
PASS:      13 / 13
WARN:      0
BLOCKERS:  0

✓ All checks pass — V312-55 Procedure/Trigger 整改已闭环
exit 0
```

JSON 状态 (`--json` 模式):
```json
{"version":"v3.12.0","pass":13,"total":13,"warn":0,"blockers":0,"status":"PASS"}
```

compat-runner 端到端输出 (Round-29 收尾时):
```
compat-runner: 22 surfaces, pass=14 unsupported=2 deferred=6 fail=0
disposition: docs/releases/v3.12.0/evidence/mysql_compat/SURFACE_DISPOSITION.md
```
新 fixture `procedure_trigger_basic` + `procedure_trigger_transactions` 均为 `PASS | executed without error`。

## 5. Evidence hash

- `V312-55-VERIFICATION.md` (本文件): sha256=`d36a4573b56a76e29890e48122125e597d21930403e75c4e289e18f7a2349b68`
- `check_v312_procedure_trigger_gate.sh` 实跑日志: sha256=`7bd7e1893d563346f70e392dc48fc9e19b8a7da1b9ee8f6cb4064eaf6b30360f`
- `SURFACE_DISPOSITION.md` 中 `procedure_trigger_basic` 行 hash: `9fb8311c77acd4d1cd7bffe2afd2312193fdbaaac6146d5858bae13b07acd5da`
- `SURFACE_DISPOSITION.md` 中 `procedure_trigger_transactions` 行 hash: `76594c5e034f43f1801a56d54f85c5749553087b6162db508332f9d701f8e953`

## 6. 关联

- 父 plan: [V312-55_PROCEDURE_TRIGGER_REMEDIATION_PLAN.md](../../V312-55_PROCEDURE_TRIGGER_REMEDIATION_PLAN.md)
- 修复范围: issue #4244, #4245
- 父 issue: #4237 (master)

## 7. README / MYSQL_COMPAT_STATUS 同步

随本 PR 同步更新:

- `README.md:154`: `存储过程` / `触发器` 行从 `UNSUPPORTED` 升级到
  `DONE / 受控基础功能`, 标注 V55A-F + V55G PR 链路
- `docs/releases/v3.12.0/MYSQL_COMPAT_STATUS.md:68`: `Stored procedures`
  从 `🔜 Deferred` 升级到 `✅ DONE (受控基础)`, 新增 `Triggers` 行
- 新增 `V312-55 受控基础范围` 章节列 Supported / Deferred v3.13 / OUT OF SCOPE 三档

## 8. 触发器与事务一致性 (来自 V55D)

关键不变式 (Round-26 已验证):
```
BEGIN;
CALL load_pair(1, 10);  -- procedure 写 base + trigger 写 audit
ROLLBACK;
-- 不变式: SELECT COUNT(*) FROM base_t; = 0
-- 不变式: SELECT COUNT(*) FROM audit_t; = 0
```
trigger body INSERT 与 base 表 INSERT 在同一事务边界内回滚 — V55G
Fixture-Transactions 真实执行通过, 在 compat-runner wire protocol
路径上重复验证。

## 9. 权限模型 fail-closed (来自 V55F)

5 路径在 root 短路之外的关闭语义:

| 路径 | 入口 | AuthManager 调用 | 失败消息 |
|---|---|---|---|
| CREATE PROCEDURE | execute_create_procedure | check_privilege(Create, ObjectRef::database("procedure:<name>")) | "Permission denied: Create on procedure:<name> for bob@localhost" |
| DROP PROCEDURE   | execute_drop_procedure   | check_privilege(Drop,   ObjectRef::database("procedure:<name>")) | 同上 (Drop) |
| CALL             | execute_call             | check_privilege(All,    ObjectRef::database("procedure:<name>")) | 同上 (All) |
| CREATE TRIGGER   | execute_create_trigger   | check_privilege(Create, ObjectRef::table("<table>")) | "Permission denied: Create on <table> for bob@localhost" |
| trigger body DML | check_body_privilege     | hook.check() → catalog.auth.check(Insert/Update/Delete, table) | "Permission denied (trigger body DML): ... for bob@localhost" |

非 root 用户 bob 在 5 条路径任一被拒后, base 表 `t1` 仍为 0 行
(`storage.scan("t1").len() == 0`)。这是 fail-closed 的金标准,
V55F evidence 已记录所有 5 条路径的 assert_perm_denied 断言。

## 10. 递归限制 (来自 V55E)

`MAX_RECURSION_DEPTH = 16` (RAII `DepthGuard` 在构造时自增计数,
析构时自减; 超过阈值抛 `SqlError::TriggerRecursionLimitExceeded`)。
t1 表自引用 BEFORE INSERT trigger 循环 → 17 层触发即拒绝,
回归 test `recursion_self_referencing_trigger_limit` PASS。