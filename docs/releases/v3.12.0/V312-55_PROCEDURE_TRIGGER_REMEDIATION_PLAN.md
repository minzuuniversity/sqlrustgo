# SQLRustGo v3.12.0 V312-55 存储过程和触发器整改计划

> **provenance:** generated_by=openclaw-minimax, generated_at=2026-08-15, commit=pending, source_repo=openclaw/sqlrustgo, branch=fix/v312-4019-3943-evidence-refresh, policy=Anti-Fabrication-Policy-v1.0 + V312-19 strict-close
>
> **目标:** 把 README/MYSQL_COMPAT_STATUS 中存储过程(`CREATE/DROP/SHOW PROCEDURE`、`CALL`)与触发器(`BEFORE/AFTER INSERT/UPDATE/DELETE` + `NEW/OLD`)的真实能力从 "UNSUPPORTED" 升级为 "DONE / 受控基础功能"。每个子项必须有合并 PR、实跑 gate、命令、退出码、输出摘要、evidence hash。

---

## 1. 背景

v3.12.0 之前 README 把存储过程与触发器标为 `UNSUPPORTED`，意味着无生产路径。Codex 在 2026-08-14 19:10 (Beta 准入硬化) 与 21:45 (Procedure/Trigger) 两轮反馈中明确指出：

- "禁止将'主路径 smoke 可用'包装成生产完成"
- "只证明 `CREATE PROCEDURE` 或 `CREATE TRIGGER` 不报错" 不算关闭
- "gate test 带 `#[ignore]`" / "gate 脚本使用 `|| true` / WARN-only" 一律不算

本计划在 v3.12.0 内交付存储过程与触发器的 **基础受控子集**,而非完整 PL/SQL。范围详见 §3。

## 2. 整改决策

| 项 | 决策 | 依据 |
|---|---|---|
| 存储过程 | **实现 v3.12 受控基础子集** (`CREATE/DROP/SHOW PROCEDURE` + `CALL p(...)` + `IN` 参数 + 过程内确定性 SQL) | Codex V312-55 反馈 + 教学场景需求 |
| 触发器 | **实现 BEFORE/AFTER row trigger 基础子集** (`INSERT/UPDATE/DELETE` + `NEW/OLD` + 事务一致性 + 递归限制) | 同上 |
| `OUT/INOUT` 参数 | DEFERRED v3.13 | 不属于基础教学范围 |
| `DEFINER` / `SQL SECURITY` 安全模型 | DEFERRED v3.13 (基础权限已覆盖) | 教学场景未强需求 |
| Dynamic SQL inside procedure | DEFERRED v3.13 | 同上 |
| 完整 MySQL 8.0 存储过程语义 (cursor, handler, condition) | OUT OF SCOPE | v3.12 受控子集以外 |

## 3. 子任务矩阵

| Issue | 主题 | Owner | Expiry | 必跑 Gate | 关闭边界 |
|---|---|---|---|---|---|
| [#4237](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4237) | V312-55 主控 (本计划) | openclaw-minimax | v3.12 RC1 | `check_v312_procedure_trigger_gate.sh` | 全部子项 DONE 或显式 DEFERRED + #4248 关闭 |
| [#4238](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4238) | V312-55A Procedure DDL 生命周期 | TBD | v3.12 RC1 | `cargo test --test stored_proc_catalog_test` + `check_v312_procedure_trigger_gate.sh` | `CREATE/DROP/SHOW PROCEDURE` + `IF EXISTS` + 大小写查找全覆盖 |
| [#4239](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4239) | V312-55B CALL + IN + 过程内确定性 SQL | TBD | v3.12 RC1 | `cargo test -p sqlrustgo-executor --test test_stored_proc` | `CALL p(args)` 真实执行过程体 SQL,IN 参数正确绑定,无参数跳过主路径 |
| [#4240](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4240) | V312-55C Trigger row semantics (NEW/OLD) | TBD | v3.12 RC1 | `cargo test --test view_procedure_trigger_e2e_test` | BEFORE 改 NEW 落库 + AFTER 触发 + NEW/OLD 上下文 fail-closed |
| [#4241](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4241) | V312-55D Trigger 事务 + WAL + recovery | TBD | v3.12 RC1 | `cargo test --test e2e_trigger_wal_recovery` + `check_v312_14_crash_recovery.sh` | 事务回滚一致 + WAL replay count/hash 一致 + 无 WAL storage 时 fail-closed |
| [#4242](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4242) | V312-55E Recursion limit | TBD | v3.12 RC1 | `cargo test --test stored_proc_catalog_test recursion` | 自触发/互触发达到深度限制时错误 + base/audit 无部分提交 |
| [#4243](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4243) | V312-55F 权限模型 | TBD | v3.12 RC1 | `cargo test --test stored_proc_catalog_test privilege` + `check_security.sh` | 无权限 CREATE/DROP/CALL + trigger body DML 全部 fail closed |
| [#4244](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4244) | V312-55G SQLLogicTest/SQL corpus 接入 | TBD | v3.12 RC1 | `check_sqllogictest_v312.sh` | 2 个 fixture (procedure_trigger_basic / _transactions) + SKIP/FAIL 全部有 issue |
| [#4245](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4245) | V312-55H E2E/wire/docs/release evidence | TBD | v3.12 RC1 | `check_docs_links.sh` + `check_v312_procedure_trigger_gate.sh` | `V312-55-VERIFICATION.md` + README 升级 + 2 reviewer |

## 4. 关闭门禁

新增 `scripts/gate/check_v312_procedure_trigger_gate.sh`,作为 v3.12 Procedure/Trigger 整改的统一硬门禁。

**设计原则(按 Codex 21:45 反馈):**

1. **FAIL 不可绕过** —— 任何 `|| true` / WARN-only 路径必须删除。
2. **`#[ignore]` 立即 FAIL** —— 不允许静默跳过。
3. **执行而非仅检查** —— 必须实跑命令、检查 exit code、检查输出关键字段,不能只看文件存在。
4. **PASS/WARN/FAIL 三态** —— 当前状态下 Procedure/Trigger 仍按设计未完成,故 **整体应 FAIL**,直至 V312-55A~55H 全部 DONE。

预期关闭路径:

```bash
$ bash scripts/gate/check_v312_procedure_trigger_gate.sh
=== V312-55 Procedure/Trigger Gate ===
  [PASS] Gate-Script-Syntax  (bash -n)
  [FAIL] V55A-Procedure-DDL  (catalog 未注册完整生命周期)
  [FAIL] V55B-Call-Execute   (CALL 仍走 unsupported 路径)
  [FAIL] V55C-Trigger-NewOld (NEW/OLD 上下文未实现)
  [FAIL] V55D-WAL-Recovery   (trigger tx + WAL recovery 未验证)
  [FAIL] V55E-Recursion      (递归限制未实装)
  [FAIL] V55F-Privilege      (权限模型未覆盖)
  [FAIL] V55G-Sqllogictest   (procedure/trigger fixture 未接入)
  [FAIL] V55H-Verification   (V312-55-VERIFICATION.md 未建)

BLOCKERS: 8
V312-55 PROCEDURE/TRIGGER GATE FAILED
exit 1
```

## 5. 关闭条件(整体)

- [ ] 全部 V312-55A~55H 子任务 issue 已合并 PR 至 `develop/v3.12.0`
- [ ] `scripts/gate/check_v312_procedure_trigger_gate.sh` 在合并后 HEAD 退出 0
- [ ] `docs/releases/v3.12.0/evidence/procedure_trigger/V312-55-VERIFICATION.md` 已建,内容含:
  - branch / commit / PR / merge commit SHA
  - 全部必跑命令(原样复制)
  - 退出码与输出关键摘要
  - evidence 文件 SHA-256
  - 不支持范围显式声明(`OUT/INOUT` / `DEFINER` / dynamic SQL)
- [ ] README 中"存储过程"行从 `UNSUPPORTED` 升级为 `DONE / 受控基础功能`,并在升级前引用 `V312-55-VERIFICATION.md` 中 gate log
- [ ] `docs/releases/v3.12.0/MYSQL_COMPAT_STATUS.md` 同步更新存储过程与触发器章节
- [ ] 至少 1 名 reviewer 或 Codex 严格复核评论确认证据

## 6. 禁止关闭条件(Anti-Pattern)

下列 **任一** 命中即视为虚假关闭,必须重做:

1. ❌ 只证明 parser 能解析 `CREATE PROCEDURE` AST
2. ❌ 只证明 `CREATE PROCEDURE` 或 `CREATE TRIGGER` 不报错
3. ❌ gate test 带 `#[ignore]` 或 `#[ignore = "..."]`
4. ❌ gate 脚本使用 `|| true` / `2>/dev/null` / WARN-only 路径绕过 FAIL
5. ❌ 缺少 WAL recovery 一致性证据
6. ❌ 缺少权限正反例
7. ❌ 缺少递归限制证据
8. ❌ 缺少 NEW/OLD 上下文 fail-closed 证据
9. ❌ 缺少 SQLLogicTest/E2E 接入证据
10. ❌ `V312-55-VERIFICATION.md` 缺 evidence hash / 缺退出码 / 缺命令原文

## 7. 验证命令(本计划阶段)

```bash
# 1. 计划文档存在
test -f docs/releases/v3.12.0/V312-55_PROCEDURE_TRIGGER_REMEDIATION_PLAN.md

# 2. Gate 脚本语法正确
bash -n scripts/gate/check_v312_procedure_trigger_gate.sh
bash -n scripts/gate/check_beta_v3.12.0.sh

# 3. 文档链接有效
bash scripts/gate/check_docs_links.sh

# 4. Gate 预期 FAIL(按设计)
bash scripts/gate/check_v312_procedure_trigger_gate.sh || echo "GATE FAILED AS DESIGNED"
```

## 8. 关联

- Issue #4220 (PARTIAL 功能整改总控) — 已通过 Round-22 Beta Gate 修复闭环
- Issue #4228 (V312 PARTIAL plan, Codex 提交)
- Issue #4248 (V312-55 plan doc 跟踪)
- Issue #4237 (V312-55 master)
- PR #4249 (V312-49..54 + Round-21 + Round-22 Beta Gate fix, 本计划后续 Round-23+ commit 接续)
