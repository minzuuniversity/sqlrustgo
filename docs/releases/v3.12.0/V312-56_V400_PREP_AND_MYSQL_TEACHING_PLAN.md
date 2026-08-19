# SQLRustGo v3.12.0 V312-56 4.0 前功能整改与 MySQL 教学能力补强计划

> **provenance:** generated_by=openclaw-minimax, generated_at=2026-08-15, commit=pending, source_repo=openclaw/sqlrustgo, branch=fix/v312-4019-3943-evidence-refresh, policy=Anti-Fabrication-Policy-v1.0 + V312-19 strict-close + V312-55 close-pattern
>
> **目标:** 把 `COMPREHENSIVE_ASSESSMENT_REPORT.md` 中标记为"有代码/有测试/有文档,但未形成生产或教学闭环"的 8 类能力,转化为 v3.12 Beta 前可教学、可验证、可降级的受控子集。每个子项必须有合并 PR、实跑 gate、命令、退出码、输出摘要、evidence hash。

---

## 1. 背景

Codex 在 2026-08-15 创建 V312-56 总控(#4250),指出除存储过程/触发器(V312-55)外,SQLRustGo 还存在一批能力文档与真实状态不一致的功能。这些功能若不在 v3.12 Beta 前定义清楚,会继续造成 README/MYSQL_COMPAT_STATUS 与真实能力不一致,也直接影响 4.0.0 生产路线和 MySQL 数据库教学场景。

本计划在 v3.12.0 内交付 8 个子项的整改方案或显式降级,详见 §3。

## 2. 整改决策总览

| 类别 | v3.12 决策 | v4.0 备注 |
|---|---|---|
| information_schema / SHOW / DESCRIBE | 实现受控子集 | 完整兼容 v4.0 |
| SQL 教学 corpus + 多 oracle | 新增 teaching corpus; 不混同 full SQLite official | v4.0 扩展 |
| Transaction / crash recovery 教学 | 已有 evidence 完整化 | 教学为主 |
| Prepared statement / wire 教学 | 已有 evidence 完整化 | 教学为主 |
| Optimizer / EXPLAIN 教学 | 受控子集 (P1,可不阻塞 Beta) | v4.0 完整 CBO |
| VIEW / CTE / MERGE disposition | CTE 受控;VIEW PARTIAL→受控;MERGE 决策二选一 | v4.0 完整 |
| Partition / FullText disposition | 决策为 DEFERRED + 与 GMP keyword retrieval 集成方案 | v4.0 + GMP |
| Beta gate / docs / evidence 集成 | gate 实跑 V312-56A~D | 持续集成 |

## 3. 子任务矩阵

| Issue | 主题 | 优先级 | Owner | Expiry | 必跑 Gate | 关闭边界 |
|---|---|---|---|---|---|---|
| [#4250](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4250) | V312-56 主控 | P0 | openclaw-minimax | v3.12 RC1 | (整体验证) `V312-56-VERIFICATION.md` + gate 实跑 | A~H 全部 DONE 或显式 DEFERRED |
| [#4251](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4251) | V312-56A Metadata/SHOW/information_schema | P0 | TBD | v3.12 RC1 | `mysql_server_e2e_test show` + `ddl_e2e_test show` + `check_v312_21_mysql_compat.sh` | SHOW CREATE TABLE / SHOW INDEX / DESCRIBE 受控;information_schema 至少 SQL 路径 |
| [#4252](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4252) | V312-56B SQL 教学 corpus + 多 oracle | P0 | TBD | v3.12 RC1 | `check_sqllogictest_v312.sh` + `check_gate_test_integrity.sh` | `tests/compat/teaching_sql_v3_12/manifest.yml` + 每个文件至少 SQLite oracle |
| [#4253](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4253) | V312-56C Transaction / crash recovery 教学 | P0 | TBD | v3.12 RC1 | `check_v312_14_crash_recovery.sh` + `wal_tx_contract_test` + `compatibility_harness` | 教学实验文档 + before/after count/hash fixture |
| [#4254](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4254) | V312-56D Prepared statement / wire 教学 | P0 | TBD | v3.12 RC1 | `check_v312_13_wire_load_data.sh` + `check_v312_21_mysql_compat.sh` + `e2e_wire_protocol` | COM_QUERY/STMTT_PREPARE/EXECUTE 正反例 + packet trace + LOAD DATA fixture |
| [#4255](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4255) | V312-56E Optimizer / EXPLAIN 教学 | **P1** | TBD | v4.0 | `cargo test -p sqlrustgo-optimizer --all-features` + `cargo test -p sqlrustgo-planner --all-features` | EXPLAIN 等价 plan dump + 5 教学 fixture + 统计信息更新前后计划变化可复现 |
| [#4256](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4256) | V312-56F VIEW/CTE/MERGE disposition | **P1** | TBD | v3.12 RC1 | `parser_e2e_test view` + `merge_e2e_test` + `check_docs_consistency.sh` | CTE 教学 fixture + VIEW PARTIAL 收口 + MERGE 决策二选一 |
| [#4257](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4257) | V312-56G Partition/FullText disposition + GMP keyword retrieval | **P1** | TBD | v4.0 + GMP | `cargo test -p sqlrustgo-storage fulltext` + `partition_e2e_test` + `check_docs_consistency.sh` | PartitionInfo 不再冲突 + FullText 决策 + GMP keyword 集成方案 |
| [#4258](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4258) | V312-56H Beta gate / docs / release evidence 集成 | P0 | TBD | v3.12 RC1 | `check_docs_links.sh` + `check_docs_consistency.sh` + `check_beta_v3.12.0.sh` | TEST_PLAN G27 + STAGE.yaml promotion_to_BETA_requires + PARTIAL_PLAN + ISSUES_PLAN + V312-56-VERIFICATION.md |

## 3A. 相邻教学入口任务: V312-57 / Issue #4359

V312-57 是独立于 V312-56A~H 的教学入口任务,用于实现 sqlite3-like 一体化 CLI,支撑 BustubX-EDU 前 4-6 周自动验收。它与 V312-56B SQL 教学 corpus 互补,但不等价:

- V312-56B 关注 SQL fixture、oracle 和 corpus 管理。
- V312-57 关注学生和自动评测实际调用的 CLI 产品形态。

V312-57 的 Beta 前边界是 week01-week04 fixture 通过 `scripts/gate/check_bustubx_edu_cli_v312.sh`;RC 前边界是 week05-week06 executor/join/aggregate fixture 通过或显式降级。不得用 V312-56B corpus 存在来关闭 V312-57,也不得把 V312-57 默认 defer 到 v3.13。

## 4. 优先级与 v3.12 Beta 阻塞关系

### Beta 必须完成 (P0)

- **V312-56A** Metadata/SHOW — README 中多个功能行引用 SHOW/DESCRIBE 能力,必须受控
- **V312-56B** SQL 教学 corpus — 教学场景入口
- **V312-56C** Transaction/crash recovery 教学 — 已有 evidence,仅需教学化补强
- **V312-56D** Prepared statement / wire 教学 — 已有 evidence,仅需教学化补强
- **V312-56H** Beta gate / docs / release evidence 集成 — 必须先于 Beta promotion 完成

### Beta 不阻塞 (P1)

- **V312-56E** Optimizer/EXPLAIN — 受控即可,CBO 完整化推到 v4.0
- **V312-56F** VIEW/CTE/MERGE disposition — CTE 受控;VIEW/MERGE 决策二选一即可
- **V312-56G** Partition/FullText + GMP keyword retrieval — DEFERRED v4.0 接受

## 5. Beta Gate 集成

`scripts/gate/check_beta_v3.12.0.sh` 必须增加或更新以下断言:

```bash
# 在 B6_BETA_PROMOTION_EVIDENCE 区域新增
check "V56-Beta-Docs-Include" \
    "grep -q 'V312-56' docs/releases/v3.12.0/TEST_PLAN.md docs/releases/v3.12.0/ISSUES_PLAN.md docs/releases/v3.12.0/PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md"

check "V56-Stage-Yaml-Beta-Requires" \
    "grep -q 'V312-56' docs/releases/v3.12.0/STAGE.yaml"

# 新增 B8_V312_56_TEACHING 部分(按设计 FAIL 直至 V312-56A~D 完成)
warn "B8-V312-56A-Metadata-Show" \
    "cargo test -p sqlrustgo-mysql-server --test mysql_server_e2e_test show -- --nocapture"
warn "B8-V312-56B-Teaching-Corpus" \
    "test -f tests/compat/teaching_sql_v3_12/manifest.yml"
warn "B8-V312-56C-Tx-Recovery-Teaching" \
    "test -f docs/releases/v3.12.0/evidence/teaching_v400/V312-56C-EXPERIMENTS.md"
warn "B8-V312-56D-Prepared-Wire-Teaching" \
    "test -f docs/releases/v3.12.0/evidence/teaching_v400/V312-56D-EXPERIMENTS.md"
warn "B8-V312-56H-Verification-Doc" \
    "test -f docs/releases/v3.12.0/evidence/teaching_v400/V312-56-VERIFICATION.md"
```

当前状态下 B8 全部 WARN,关闭后转 PASS。

## 6. 关闭条件(整体)

- [ ] 全部 V312-56A~56H 子任务 issue 已合并 PR 至 `develop/v3.12.0`,或显式 DEFERRED + owner + expiry
- [ ] `scripts/gate/check_beta_v3.12.0.sh` 包含 V312-56A~56D 验证(实跑而非仅检查)
- [ ] `docs/releases/v3.12.0/ISSUES_PLAN.md` / `TEST_PLAN.md` / `PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md` / `COMPREHENSIVE_ASSESSMENT_REPORT.md` 与实际状态一致
- [ ] `docs/releases/v3.12.0/STAGE.yaml` 中 `promotion_to_BETA_requires` 包含 V312-56A~56D
- [ ] `docs/releases/v3.12.0/evidence/teaching_v400/V312-56-VERIFICATION.md` 已建,内容含:
  - branch / commit / PR / merge commit SHA
  - 全部必跑命令(原样复制)
  - 退出码与输出关键摘要
  - evidence 文件 SHA-256
  - 不支持范围显式声明
- [ ] #4250 总控评论引用本计划 + verification doc

## 7. 禁止关闭条件(Anti-Pattern)

1. ❌ 只证明 parser 能解析 SHOW/DESCRIBE/EXPLAIN AST
2. ❌ 教学 corpus 用 `#[ignore]` 静默通过
3. ❌ MERGE 仍走 `execute()` unsupported 路径但 README 写 PARTIAL
4. ❌ VIEW 仅保存 definition 但 README 写 PARTIAL/DONE
5. ❌ PartitionInfo 与 `ALTER TABLE SET PARTITIONED BY` 状态不一致
6. ❌ FullTextIndex 测试与 MATCH/AGAINST 需求无明确关系
7. ❌ STAGE.yaml 包含 V312-56 但 Beta Gate 不实跑
8. ❌ `V312-56-VERIFICATION.md` 缺 evidence hash / 缺退出码 / 缺命令原文

## 8. 验证命令(本计划阶段)

```bash
# 1. 计划文档存在
test -f docs/releases/v3.12.0/V312-56_V400_PREP_AND_MYSQL_TEACHING_PLAN.md

# 2. Beta Gate 仍 PASS(含本计划对接后预期不变)
bash scripts/gate/check_beta_v3.12.0.sh

# 3. V312-55 Gate FAIL by design(本 Round-23 闭环但不阻塞)
bash scripts/gate/check_v312_procedure_trigger_gate.sh || echo "V312-55 GATE FAILED AS DESIGNED"

# 4. 文档链接有效
bash scripts/gate/check_docs_links.sh
```

## 9. 关联

- Issue #4220 (PARTIAL 整改总控) — Round-22 已闭环 B1_CLIPPY/B1_FMT
- Issue #4228 (V312 PARTIAL plan, Codex 提交)
- Issue #4237 (V312-55 master,Procedure/Trigger)
- Issue #4248 (V312-55 plan doc)
- Issue #4250 (V312-56 master,本计划)
- Issue #3887 (V312-MASTER) — 本计划应在 #3887 总控评论同步
- PR #4249 (V312-49..54 + Round-21 + Round-22 + Round-23 plan docs)
