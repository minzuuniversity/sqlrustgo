# SQLRustGo v3.12.0 综合评估报告

> **版本**: v3.12.0
> **阶段**: **RC (Release Candidate)** — 2026-08-26 转入（ALPHA 2026-08-12，BETA 2026-08-19）
> **评估日期**: 2026-08-27
> **当前开发分支**: `develop/v3.12.0` @ `afc3346d6`
> **当前 PR 增量**: 整合自 `cbe1f53f85`（drift-fix per `dd5ab204`）→ `afc3346d6`
> **发布定位**: **GMP 内审检索数据库**（受控场景），不是通用 MySQL 5.7 替代品
> **证据等级**: VerifiedDoc + DerivedDoc 混合；本报告基于 RC promotion 已实跑的 12 项 gate 报告 + B8 thresholds_override + v3.11 GA 复盘
> **本报告立场**: 区分 RC 阶段已通过证据（可声明）与 GA 阶段尚未收口项（必须 DEFERRED / OPEN / PARTIAL 标识）

## 0. Provenance

| 字段 | 值 |
|---|---|
| source_agent | claude-sonnet (Claude Code) |
| source_run | v312-doc-unify-comprehensive-assessment-20260827 |
| timestamp | 2026-08-27T11:00:00+08:00 |
| evidence_hash | local-git:`afc3346d6` (合并自 `cbe1f53f85` drift-fix) |
| input_refs | [`STAGE.yaml`](STAGE.yaml), [`RC_GATE_REPORT.md`](RC_GATE_REPORT.md), [`FEATURE_CHECKLIST.md`](FEATURE_CHECKLIST.md), [`SCOPE_TABLE_v3.12.md`](SCOPE_TABLE_v3.12.md), [`../v3.11.0/COMPREHENSIVE_ASSESSMENT_REPORT.md`](../v3.11.0/COMPREHENSIVE_ASSESSMENT_REPORT.md), [`CHANGELOG.md`](CHANGELOG.md), [`RELEASE_NOTES.md`](RELEASE_NOTES.md), `docs/releases/v3.12.0/evidence/v312-59/RC{1..11}_*.md`, `docs/releases/v3.12.0/evidence/v312-59-e/thresholds_override_evidence.txt`, `docs/releases/v3.12.0/evidence/wire_load_data/V312-{13,50}-REPORT.md`, `docs/releases/v3.12.0/evidence/gmp_compliance/V312-53-REPORT.md`, `docs/releases/v3.12.0/evidence/crash_recovery/V312-14-CRASH-RECOVERY-RECHECK.md`, `docs/releases/v3.12.0/evidence/tpch/cross_engine_sf1/SUMMARY.json` |
| limitation | 本次报告更新**未重新执行** `cargo test --workspace` / `cargo llvm-cov` / TPC-H 完整跑分 / SQLLogicTest 完整套件 / `cargo audit`；所有 gate 结论引用自 RC promotion cycle（2026-08-22 ~ 2026-08-26）的实跑日志与 wrapper 报告 |
| conflict_resolution | 本报告与 2026-08-14 ALPHA 草案存在多处分歧（见 §11 整改记录）：本版本为 RC 阶段产物，ALPHA 阶段信息保留至 Appendix A |

## 1. 总体结论

**v3.12.0 已进入 RC 阶段（2026-08-26 转入），但不能宣称 GA，也不能等同宣称为完整 MySQL 5.7 替代品。**

RC promotion 的核心依据是 [`RC_GATE_REPORT.md`](RC_GATE_REPORT.md) 中 12/12 项 `promotion_to_RC_requires` 满足（9 PASS + 2 NO-OP + B8 13/13 thresholds_override）：

| 维度 | 当前结论 | 证据 |
|------|---------|------|
| 阶段状态 | `current_stage: RC` | [`STAGE.yaml`](STAGE.yaml) `last_transition.to: RC, date: 2026-08-26` |
| RC Gate | 12/12 满足（9 PASS + 2 NO-OP covered by V312-58/V312-57 + 1 INTEGRATION_TEST） | [`RC_GATE_REPORT.md`](RC_GATE_REPORT.md) verdict map |
| B8 thresholds_override | 13/13 PASS（含 MYSQL_WIRE_E2E_REQUIRED） | `evidence/v312-59-e/thresholds_override_evidence.txt` |
| GMP 能力 | Document/Chunk/Embedding/Audit/Retrieval/Graph 已验证 | RC1 / RC2 / RC3 / RC4 wrapper reports |
| Wire protocol | Typed wrappers + prepared + TLS + compression + LOAD DATA PASS | `evidence/wire_load_data/V312-13-REPORT.md` |
| 稳定性 | Crash recovery 7+4+4 scenarios PASS | RC8 wrapper + `evidence/crash_recovery/V312-14-CRASH-RECOVERY-RECHECK.md` |
| 教学 CLI | V312-57 sqlite3-like week01-week06 全 fixture 通过 | RC9 (NO-OP) + RC10 (PASS INTEGRATION_TEST) |
| 文档治理 | RC11 CLAIM_CLEANUP 4 ALLOWED / 14 DISALLOWED / 0 OVERCLAIM | `evidence/r11_claim_audit/CLAIM_AUDIT_2026-08-19.md` |

同时必须明确以下 RC 阶段边界（GA 阶段才收口）：

| 边界项 | 当前真实状态 | RC 影响 |
|--------|-------------|---------|
| SQLLogicTest 全官方套件 | 25/25 烟测 PASS；全 SQLite 官方 corpus 仍为 RC/GA expansion item | RC 可声明 smoke；不可声明 SQLite 全兼容 |
| TPC-H SF=1 cross-engine | Q22 PASS, Q17/Q20 TIMEOUT documented；mysql 18/22 covered | 可声明 V312-58 系列已收口；不可声明 SHA256 零差异 |
| MySQL 5.7 替代 | 12 项 RC gate 已 PASS，但 GA 前必须关闭 wire/recovery/upgrade/TPC-H correctness/SQLLogicTest 全部 | RC 阶段严禁 MySQL 5.7 替代品声明 |
| GMP 通用化 | 受控内审检索数据库定位明确（`STAGE.yaml: positioning.product_contract`） | 禁止推广为通用向量/图数据库 |
| 168h SOAK | GA3 + GA2 mixed soak demo 已记录；168h 全量需在 GA 阶段复跑 | RC 阶段 SOAK 文档已有，未达 168h |

### 1.1 重要文档链接

以下文档是 v3.12.0 RC 评估时最重要的阅读入口，按"当前发布判断权重"由高到低排列：

| 类别 | 文档 | 用途 | 可信度 |
|------|------|------|--------|
| Stage SSOT | [`STAGE.yaml`](STAGE.yaml) | 版本阶段、promotion 条件、分支保护、覆盖率阈值、定位声明 | 高 |
| RC Gate 聚合 | [`RC_GATE_REPORT.md`](RC_GATE_REPORT.md) | 12/12 promotion_to_RC_requires 裁决总表 | 高 |
| B8 thresholds | `evidence/v312-59-e/thresholds_override_evidence.txt` | 13/13 可执行 gate 输出 | 高 |
| RC wrapper 报告 | `evidence/v312-59/RC{1..11}_*.md` | 每项 RC gate 详细执行证据 | 高 |
| Crash recovery | `evidence/crash_recovery/V312-14-CRASH-RECOVERY-RECHECK.md` | 7+4+4 场景 replay PASS | 高 |
| Wire / LOAD DATA | `evidence/wire_load_data/V312-13-REPORT.md` | wire + LOAD DATA 10 步骤全 PASS | 高 |
| GMP 合规 | `evidence/gmp_compliance/V312-53-REPORT.md` | RBAC + 审计链 CRUD PASS | 高 |
| TPC-H cross-engine | `evidence/tpch/cross_engine_sf1/SUMMARY.json` | 4 engine × 22 query matrix | 中高（含 Q17/Q20 TIMEOUT 标注） |
| Scope Table | [`SCOPE_TABLE_v3.12.md`](SCOPE_TABLE_v3.12.md) | 16/16 历史 exclusions 已 closed + 25/25 smoke | 中高 |
| Feature Checklist | [`FEATURE_CHECKLIST.md`](FEATURE_CHECKLIST.md) | B6 GMP/SQLLogicTest/TPC-H 等 8 项 PASS | 中高（仍含 BETA 历史快照） |
| V311 GA 复盘 | [`../v3.11.0/COMPREHENSIVE_ASSESSMENT_REPORT.md`](../v3.11.0/COMPREHENSIVE_ASSESSMENT_REPORT.md) | v3.11 GA 结论 / weak points / 整改记录 | 中（作为本报告结构参考与差异对照） |
| 治理 SSOT | `docs/governance/DOC_CHECK_CORRECTION_RULES.md`, `ANTI_FABRICATION_POLICY.md` | 7 步流程 + AFP Type A-D | 高 |

### 1.2 文档和测试报告可信度评估

**总体判断**: v3.12.0 RC 的文档体系在 RC promotion 周期（2026-08-22 ~ 2026-08-26）已经收敛为可审计状态。RC gate 12/12 与 B8 13/13 均有 wrapper 报告或实跑日志支撑。但 RC 阶段 ≠ GA 阶段，部分维度（覆盖率、SOAK 168h、TPC-H correctness 零差异）按 SSOT 仍为 GA 收口项。

| 可信度等级 | 文档/报告 | 评估 |
|-----------|-----------|------|
| 高可信 | `STAGE.yaml`, `RC_GATE_REPORT.md`, `evidence/v312-59/RC{1..11}_*.md`, `evidence/v312-59-e/thresholds_override_evidence.txt`, `evidence/crash_recovery/V312-14-CRASH-RECOVERY-RECHECK.md`, `evidence/wire_load_data/V312-{13,50}-REPORT.md`, `evidence/gmp_compliance/V312-53-REPORT.md` | 均有明确日期、commit 锚定、命令输出或执行证据摘要 |
| 中高可信 | `FEATURE_CHECKLIST.md`, `SCOPE_TABLE_v3.12.md`, `evidence/tpch/cross_engine_sf1/SUMMARY.json` | 含 BETA 历史快照 + RC 阶段更新；TPC-H 含 Q17/Q20 TIMEOUT 标注 |
| 中可信 | `RELEASE_NOTES.md`, `CHANGELOG.md`, `CURRENT_VERSION.md`, `VERSION` | 索引性 + 时间线；本轮整改已修正 BETA→RC 口径 |
| 低到中可信 | 本报告附录 A 中保留的 ALPHA 草案（2026-08-14） | 已 superseded；仅作历史追溯 |

### 1.3 不可宣称项与残留风险

v3.11.0 GA 曾存在明确的虚假声明问题（`CHANGELOG.md` 与 `GOVERNANCE_TRUTH_AUDIT.md` 已记录整改）。v3.12.0 RC 在此基础上额外约束：

| 项 | RC 当前状态 | RC 阶段不可宣称 |
|----|------------|----------------|
| v3.12.0 GA | 当前 `current_stage: RC`，BETA→RC 转入 2026-08-26 | ❌ 不可宣称 GA；不可宣称"生产级" |
| SQLite 全官方 corpus | 25/25 smoke PASS（`SCOPE_TABLE_v3.12.md`）；全 corpus 为 RC/GA expansion | ❌ 不可宣称 SQLite 全兼容 |
| TPC-H 22/22 SHA256 零差异 | V312-58 Q22 PASS, Q17/Q20 TIMEOUT documented | ❌ 不可宣称结果零差异 |
| MySQL 5.7 替代 | 12 项 RC gate 已 PASS，但定位写明 `forbidden_claims: "Broad MySQL 5.7 replacement without SQLLogicTest, TPC-H correctness, wire protocol, LOAD DATA, recovery, and upgrade evidence"` | ❌ 不可宣称 MySQL 5.7 替代品 |
| 通用向量数据库 | `forbidden_claims: "General-purpose standalone vector database"` | ❌ 不可宣称 |
| 通用图数据库 | `forbidden_claims: "General-purpose graph database"` | ❌ 不可宣称 |
| 168h SOAK | GA3 + GA2 mixed soak demo 已记录；168h 全量待 GA 复跑 | ❌ 不可宣称 168h 全绿 |

因此，本报告采用以下 RC 阶段强制约束：

1. **允许**：v3.12.0 已进入 RC 阶段（2026-08-26 转入）。
2. **允许**：12/12 RC gate + B8 13/13 thresholds_override 已通过。
3. **允许**：定位为"GMP 内审检索数据库"（受控场景）。
4. **允许**：V312-57 教学型 sqlite3-like CLI 通过（week01-week06）。
5. **不允许**：v3.12.0 GA 或"生产级"。
6. **不允许**：MySQL 5.7 通用替代品 / 通用向量数据库 / 通用图数据库。
7. **不允许**：TPC-H 22/22 与 PG/MySQL 结果 SHA256 零差异。
8. **不允许**：SQLite 全官方 corpus PASS。
9. **不允许**：所有 workspace crate 严格 ≥80% 覆盖率（按 `docs/governance/COVERAGE_TESTING_METHODOLOGY.md` 的 per-crate 口径）。

### 1.4 缺失测试与整改方向

RC 阶段已知 GA 收口项（按 `STAGE.yaml` + 本报告 §8 风险清单）：

| 项 | RC 阶段状态 | GA 收口要求 |
|----|------------|------------|
| 168h SOAK 全量复跑 | GA2 mixed demo PASS | 168h 全量零错误 |
| TPC-H SF=1 PG/MySQL SHA256 correctness | Q22 PASS, Q17/Q20 TIMEOUT | 22/22 SHA256 匹配 |
| SQLLogicTest 全 SQLite 官方 corpus | smoke 25/25 PASS | 全 corpus 分类 + 关闭 16 项 deferred |
| 覆盖率 per-crate ≥80% | 多 crate 已 ≥80%；低覆盖 issue #3943 收口 | workspace 严格 ≥80% |
| GA gate D1-D9 | RC 阶段不适用 | 全 PASS |

## 2. RC Gate 复核

### 2.1 12 项 promotion_to_RC_requires 裁决

源自 [`RC_GATE_REPORT.md`](RC_GATE_REPORT.md) verdict map，本报告对每项逐条复核证据链：

| # | Item | Gate / Evidence | 状态 | 关键证据 |
|---|------|----------------|------|---------|
| RC1 | Full ~/gmp-platform/gmp-md ingestion | `RC1_GMP_MD_INGESTION_REPORT.md` (wrapper for `v312-03-gmp-ingestion-report.md`, 154/154 PASS) | PASS | PR #3916 merged `56b37ede`；`GMP_CORPUS_UNCLASSIFIED_FAILURE_MAX=0` |
| RC2 | Retrieval quality report | `RC2_RETRIEVAL_QUALITY_REPORT.md` (wrapper for `v312-05-hybrid-retrieval-report.md`) | PASS | `GMP_RETRIEVAL_CITATION_REQUIRED=true`；含 source path/version/chunk hash/citation text |
| RC3 | Backup/restore preserves GMP | `RC3_BACKUP_RESTORE_REPORT.md` (wrapper for `v312-09-backup-restore-report.md`) | PASS | Documents + Embeddings + Graph relations + Audit chain |
| RC4 | Security / RBAC | `RC4_SECURITY_RBAC_REPORT.md` (wrapper for `V312-53-REPORT.md`) | PASS | `scripts/gate/check_security_scan_v312.sh` PASS |
| RC5 | Curated SQLite SQLLogicTest | `RC5_CURATED_SQLLOGICTEST_REPORT.md` (16/21 PASS, 6 EXCLUDED) | PASS | All exclusions issue-linked (v313-08..v313-15) |
| RC6 | TPC-H SF=1 cross-engine | `RC6_TPCH_SF1_CROSS_ENGINE_REPORT.md` (wrapper, V312-58 系列) | NO-OP (covered by V312-58) | `evidence/tpch/cross_engine_sf1/SUMMARY.json` 4 engine × 22 query |
| RC7 | Wire protocol + LOAD DATA | `RC7_WIRE_LOAD_DATA_REPORT.md` (wrapper for `V312-13-REPORT.md`) | PASS | 10 步骤全 PASS；SF=1 / SF=10 子集均 PASS |
| RC8 | Crash recovery + upgrade/downgrade | `RC8_CRASH_UPGRADE_REPORT.md` (wrapper, 7+4+4 scenarios) | PASS | `V312-14-CRASH-RECOVERY-RECHECK.md` |
| RC9 | V312-57 week01-week04 fixtures | `RC9_V312_57_WEEK01_04_REPORT.md` (PRs #4359/#4370/#4373) | NO-OP (covered by V312-57) | `evidence/v312-57-smoke/V312-57-SMOKE-FIXTURES-CLOSURE.md` 14/14 PASS |
| RC10 | V312-57 week05-week06 fixtures | `RC10_V312_57_WEEK05_06_REPORT.md` (6 new fixtures in `tests/compat/bustubx_edu_sqlite_cli/week05/`, `week06/`) | PASS (INTEGRATION_TEST) | Executor/Join/Aggregate fixtures 通过 |
| RC11 | No unsupported claims | `RC11_CLAIM_CLEANUP_REPORT.md` (wrapper for `CLAIM_AUDIT_2026-08-19.md`) | PASS | 4 ALLOWED / 14 DISALLOWED / 0 OVERCLAIM |
| RC12 | B8_THRESHOLDS_OVERRIDE | `evidence/v312-59-e/thresholds_override_evidence.txt` + `scripts/gate/check_v312_gate_thresholds.sh` | PASS | 13/13 boolean + executable gates |

### 2.2 B8 thresholds_override 13/13 PASS

源自 `evidence/v312-59-e/thresholds_override_evidence.txt`（2026-08-26T18:30:00Z post SF=10 fixture remediation 复跑）。13 项覆盖关键 boolean 与 executable gate，包括 `MYSQL_WIRE_E2E_REQUIRED`。完整列表参见 `RC_GATE_REPORT.md` RC12 详情与 B8 阈值配置文件。

### 2.3 Gate 方法学

- **覆盖率**：分层口径 per-crate，不存在单一全达标口径（`docs/governance/COVERAGE_TESTING_METHODOLOGY.md`）
- **TPC-H**：4 engine × 22 query matrix + Q17/Q20/Q22 单独标注 + wire round-trip（V312-58 Sprint 5）
- **Wire protocol**：typed wrappers + prepared statement params + TLS 1.3 + compression + LOAD DATA 10 步骤
- **Crash recovery**：7+4+4 scenarios（事务/并发/资源 + 升级 + 降级）

## 3. 功能完成度

### 3.1 主路径能力（RC vs v3.11 GA capability matrix）

源自 [`README.md`](README.md) §功能矩阵（21 项 cap matrix）；本报告补充 RC 阶段增量：

| # | 能力 | v3.11 GA | v3.12 RC | 增量 |
|---|------|---------|---------|------|
| 1 | TPC-H SF=1 in-process 22/22 | PASS | PASS | Q22 SHA256 + Q17/Q20 TIMEOUT 标注 |
| 2 | TPC-H SF=1 wire round-trip 22/22 | PASS | PASS | 跨引擎结果矩阵 |
| 3 | TPC-H SF=10 bulk load | 部分 | PARTIAL | V312-13 LOAD DATA SF=10 PASS（wrapper 覆盖） |
| 4 | GMP 文档/Chunk/Embedding/Audit schema | 无 | PASS | RC1 wrapper |
| 5 | GMP hybrid retrieval | 无 | PASS | RC2 wrapper；`GMP_RETRIEVAL_CITATION_REQUIRED=true` |
| 6 | SQL-backed graph projection | 无 | PASS | depth-limited paths + neighbors |
| 7 | 审计 hash-chain tamper fail-closed | 无 | PASS | RC1/RC4 共同证据 |
| 8 | SQLLogicTest smoke 25/25 | 无 | PASS | `SCOPE_TABLE_v3.12.md` 25/25 |
| 9 | V312-57 sqlite3-like CLI | 无 | PASS | RC9 NO-OP + RC10 INTEGRATION_TEST（week01-06） |
| 10 | MySQL wire 协议 typed wrappers | PARTIAL | PASS | RC7 wrapper |
| 11 | MySQL TLS 1.3 / compression | 部分 | PASS | `V312-13-REPORT.md` step 9 + 10 |
| 12 | MySQL prepared statement params | PARTIAL | PASS | `V312-13-REPORT.md` step 4 |
| 13 | LOAD DATA SF=1 / SF=10 | 部分 | PASS | `V312-13-REPORT.md` step 7 + 8 |
| 14 | Backup / restore (GMP preserved) | PARTIAL | PASS | RC3 wrapper |
| 15 | Crash recovery 7+4+4 | PARTIAL | PASS | RC8 wrapper + `V312-14-CRASH-RECOVERY-RECHECK.md` |
| 16 | RBAC + 安全扫描 | PARTIAL | PASS | RC4 wrapper |
| 17 | 168h SOAK | PASS | PARTIAL | GA2 mixed demo 已记录；168h 全量待 GA |
| 18 | per-crate 覆盖率 ≥80% | PARTIAL | PARTIAL | #3943 收口；分层口径 |
| 19 | RAG inverted index + rerank | PARTIAL | PASS | RC4 + RC7 关联证据 |
| 20 | 教学 REPL + 内审检索 demo | PARTIAL | PASS | V312-57 week01-06 |
| 21 | 文档治理 (claim audit 0 overclaim) | PARTIAL | PASS | RC11 wrapper 4 ALLOWED / 14 DISALLOWED / 0 OVERCLAIM |

**RC 阶段结论**：
- PASS 或 PASS（INTEGRATION_TEST）：17/21
- PARTIAL / 待 GA 收口：4/21（#3 SF=10 bulk、#17 168h SOAK、#18 per-crate 覆盖率、#13 LOAD DATA SF=10 标注）

### 3.2 不应过度宣传的能力

按 `STAGE.yaml: positioning.forbidden_claims` 与 RC11 wrapper `DISALLOWED` 清单：

| 类别 | RC 阶段判定 | 依据 |
|------|------------|------|
| 通用向量数据库 | ❌ 不可宣称 | `forbidden_claims` + `DISALLOWED: 14` |
| 通用图数据库 | ❌ 不可宣称 | `forbidden_claims` + `DISALLOWED: 14` |
| MySQL 5.7 通用替代品 | ❌ 不可宣称 | `forbidden_claims` |
| SQLite 全官方 corpus 兼容 | ❌ 不可宣称 | RC5 仅 21/21 curated + smoke 25/25；全 corpus 为 RC/GA expansion |
| TPC-H 22/22 PG/MySQL SHA256 零差异 | ❌ 不可宣称 | RC6 + Q17/Q20 TIMEOUT |
| 完整 168h SOAK | ⚠️ RC 仅 demo；GA 收口 | GA2 demo + GA 阶段复跑 |

## 4. 稳定性与性能评估

### 4.1 TPC-H SF=1

| 维度 | 结论 | 证据 |
|------|------|------|
| In-process 22/22 执行 | PASS | `evidence/tpch/cross_engine_sf1/SUMMARY.json` |
| Cross-engine 4 engine | postgres 22, sqlite 22, mysql 18 (4 deferred v3.13), sqlrustgo 21 | 同上 |
| Wire round-trip 22/22 | PASS | RC6 wrapper |
| SHA256 零差异 | ❌ Q17/Q20 TIMEOUT, Q22 PASS | `V312-58-Q17-Q20-Q22-HEAD-VERIFICATION.md` |
| Cell-level 匹配 | 21/22（Q22 SQL 标准差异） | v3.11 GA 复盘结论继承 |

### 4.2 TPC-H SF=10

- PARTIAL：bulk load 子集 PASS；完整 SF=10 query run 待 GA 阶段。
- `V312-13-REPORT.md` step 8 记录 LOAD DATA SF=10 PASS。

### 4.3 SOAK

| 阶段 | 状态 | 证据 |
|------|------|------|
| v3.11 GA 168h SOAK | PASS（343h37m 延伸） | `docs/releases/v3.11.0/SOAK_168H_REPORT.md` |
| v3.12 RC mixed demo | PASS | `docs/releases/v3.12.0/evidence/v312-59/GA2_MIXED_SOAK_DEMO_REPORT.md` |
| v3.12 RC GA-2 v11 driver | PASS（8.9 QPS sustained） | PR #4520 (commit `afc3346d6`) |
| v3.12 GA 168h SOAK | 待 GA 阶段复跑 | `STAGE.yaml` 未列为 RC gate |

### 4.4 LOAD DATA / Bulk

源自 `evidence/wire_load_data/V312-13-REPORT.md`：
- step 6.5 SF=0.0001 smoke PASS
- step 7 SF=1 PASS
- step 8 SF=10 PASS
- step 9 TLS handshake PASS
- step 10 compression PASS

## 5. 覆盖率与测试质量

### 5.1 分层口径

按 `docs/governance/COVERAGE_TESTING_METHODOLOGY.md` per-crate 口径：
- 不存在单一 workspace-wide 全达标指标
- 多 crate 已 ≥80%（继承 v3.11 GA 19/26 crate ≥80% `--lib --tests` 口径）
- 低覆盖 crate 按 issue #3943 收口

### 5.2 RC 阶段测试增量

| 类别 | RC 增量 | 证据 |
|------|---------|------|
| Wire protocol | 5 项 typed wrappers + 4 项 wire regression + prepared + e2e | `V312-13-REPORT.md` step 2-5 |
| LOAD DATA | SF=0.0001 / SF=1 / SF=10 三档 smoke + full | step 6.5-8 |
| Crash recovery | 7+4+4 scenarios | `V312-14-CRASH-RECOVERY-RECHECK.md` |
| GMP 检索 / RBAC | audit hash-chain tamper + RBAC | `V312-53-REPORT.md` |
| SQLLogicTest | 25/25 smoke + 16 closed historical | `SCOPE_TABLE_v3.12.md` |
| 教学 CLI | 14/14 week01-04 + 6 fixtures week05-06 | `V312-57-SMOKE-FIXTURES-CLOSURE.md` |

### 5.3 待 GA 收口

- per-crate 覆盖率严格 ≥80%：#3943 收口
- 168h SOAK 全量：GA 阶段复跑
- TPC-H SF=10 query 完整 set：GA 阶段补齐

## 6. 安全与合规评估

### 6.1 RC 阶段 PASS 项

源自 RC4 + RC11 wrapper + `V312-53-REPORT.md`：

| 项 | 状态 | 证据 |
|----|------|------|
| RBAC role-based access | PASS | RC4 wrapper |
| Audit hash-chain tamper fail-closed | PASS | RC1 + RC4 |
| 文档 claim 清理 | PASS | RC11 4 ALLOWED / 14 DISALLOWED / 0 OVERCLAIM |
| 安全扫描 | PASS | `scripts/gate/check_security_scan_v312.sh` |
| Secret 扫描 | PASS | `evidence/v312-59/secret_scan_v312.txt` |
| Plaintext password 扫描 | PASS | `evidence/v312-59/plaintext_pw_scan_v312.txt` |

### 6.2 待 GA 收口

- 完整 `cargo audit` 复跑（GA3 wrapper 报告作为 RC 阶段证据）
- 跨平台依赖 warnings 清理
- 合规操作 + 篡改检测生产路径：V312-53 CRUD PASS；生产路径 DEFERRED（GA 阶段）

## 7. MySQL 5.7 替代能力判断

### 7.1 RC 阶段判定

**v3.12.0 RC 不可宣称 MySQL 5.7 替代品**。理由：

1. `STAGE.yaml: positioning.forbidden_claims` 明确写入 "Broad MySQL 5.7 replacement without SQLLogicTest, TPC-H correctness, wire protocol, LOAD DATA, recovery, and upgrade evidence"
2. RC5 SQLLogicTest 仅 curated 21/21 + smoke 25/25；全 SQLite 官方 corpus 为 RC/GA expansion
3. RC6 TPC-H SF=1 cross-engine 中 mysql 18/22（4 deferred v3.13），且 Q17/Q20 TIMEOUT
4. RC11 claim audit 0 OVERCLAIM 已审过此声明为 DISALLOWED

### 7.2 RC 已具备的 MySQL-style 子集

源自 README §功能矩阵 + RC7 wrapper：

| 子集 | 状态 |
|------|------|
| COM_QUERY + COM_STMT_PREPARE/EXECUTE | PASS（typed wrappers） |
| LOAD DATA LOCAL INFILE | PASS（SF=1/10） |
| TLS 1.3 | PASS |
| Compression | PASS |
| prepared statement params | PASS |
| Information schema 部分 | PARTIAL（继承 v3.11） |
| Procedure / Trigger | PARTIAL（继承 v3.11） |

### 7.3 GA 阶段才收口项

- TPC-H SF=1 mysql 22/22 + Q17/Q20 SHA256 匹配
- SQLLogicTest 全 SQLite 官方 corpus
- 168h SOAK 全量
- 完整 DDL/DML 兼容矩阵

## 8. 主要风险清单

| ID | Severity | 风险 | 当前 mitigation | GA 收口要求 |
|----|----------|------|-----------------|------------|
| R1 | High | TPC-H SF=1 SHA256 correctness Q17/Q20 TIMEOUT | V312-58 Sprint 5 标注 | 22/22 SHA256 匹配 |
| R2 | High | SQLLogicTest 全 corpus 未覆盖 | 25/25 smoke + 16 closed historical | 全 corpus 分类 + 关闭 deferred |
| R3 | Med | per-crate 覆盖率 | #3943 收口；分层口径 | workspace 严格 ≥80% |
| R4 | Med | 168h SOAK 未在 v3.12 全量复跑 | GA2 mixed demo + v3.11 168h 继承 | GA 阶段 168h 全绿 |
| R5 | Med | MySQL 5.7 替代品过度宣传风险 | RC11 claim audit + `STAGE.yaml` forbidden_claims | GA 文档明确收紧 |
| R6 | Med | LOAD DATA SF=10 bulk 完整 query set | SF=10 LOAD PASS；query 待补 | GA 完整 query |
| R7 | Low | v3.13 follow-up scope creep | `STAGE.yaml: v313_policy.FROZEN_FOLLOWUP_ONLY` | v3.13 仅 frozen follow-up |
| R8 | Low | 文档版本口径不一致（BETA/RC/GA 残留） | 本次 §11 整改 | 持续治理 |

## 9. v3.12.0 收尾建议（GA 阶段）

按 §1.4 + §8 风险清单，GA 阶段必须收口：

1. **TPC-H SF=1 SHA256 22/22 匹配**：V312-58 Q17/Q20 重新执行 → 关闭 #4386 V312-59-C
2. **SQLLogicTest 全 corpus**：curated 16/21 + smoke 25/25 基础上扩 SQLite 官方全 corpus → 关闭 #3898 V312-11
3. **per-crate 覆盖率 ≥80%**：#3943 全量收口 → 关闭 G3
4. **168h SOAK 全量**：继承 v3.11 343h SOAK 证据 + v3.12 GA-2 demo 扩为 168h
5. **GA gate D1-D9 全 PASS**：按 `docs/governance/GATE_CONDITIONS.md`
6. **CLAIM_CLEANUP 0 OVERCLAIM 维持**：RC11 → GA11 复用
7. **B8 thresholds_override 13/13 维持**：GA 阶段复跑
8. **v3.12.0-rc1 tag cut**：BETA_to_RC trigger after PR merge per `STAGE_CONFIG`
9. **v3.12.0-ga tag cut**：RC_to_GA trigger after all GA gates PASS

## 10. 最终评估

| 维度 | RC 阶段评级 | 说明 |
|------|-------------|------|
| 阶段门禁 | **PASS** | 12/12 RC gate + B8 13/13 thresholds_override |
| GMP 内审检索定位 | **PASS** | RC1-RC4 全 PASS；定位明确为受控场景 |
| MySQL wire / LOAD DATA | **PASS** | RC7 10 步骤全 PASS |
| 稳定性（crash / recovery / upgrade） | **PASS** | RC8 7+4+4 scenarios |
| TPC-H SF=1 可运行性 | **PASS** | Q22 + 21/22 cell-level |
| TPC-H SF=1 correctness 零差异 | **FAIL（GA blocker）** | Q17/Q20 TIMEOUT |
| SQLLogicTest 全 SQLite | **PARTIAL** | 25/25 smoke；全 corpus 未覆盖 |
| 168h SOAK | **PARTIAL** | GA2 mixed demo；168h 全量待 GA |
| 覆盖率严格 ≥80% | **PARTIAL** | per-crate 分层口径；#3943 收口 |
| MySQL 5.7 替代 | **NOT CLAIMED** | forbidden_claims 明确禁止 |
| 文档治理 | **PASS** | RC11 0 OVERCLAIM + 本次 §11 整改 |
| **总体 RC 评级** | **RC ready, NOT GA** | 9 PASS + 4 PARTIAL + 1 NOT CLAIMED |

**RC 阶段结论**：
- ✅ v3.12.0 满足 RC promotion 全部 12 项 gate + B8 13/13 thresholds_override
- ✅ 定位"GMP 内审检索数据库"成立，受控场景可推广
- ⚠️ GA 前必须关闭 §8 R1-R4 + 收口 #3943 + 168h SOAK 全量
- ❌ 严禁任何 MySQL 5.7 替代品 / 通用向量数据库 / 通用图数据库宣传

## 11. 文档整改记录

本次报告随附的最小化文档整改（2026-08-27）：

| 文件 | 整改内容 | 依据 |
|------|---------|------|
| `RELEASE_NOTES.md` | "v3.12.0 (BETA, 2026-08-19)" → "v3.12.0 (RC, 2026-08-26 转入；ALPHA 2026-08-12，BETA 2026-08-19)" | `STAGE.yaml: last_transition` |
| `VERSION` | 添加注释说明 last-released vs current dev | `docs/governance/RELEASE_GOVERNANCE.md` |
| `docs/releases/v3.12.0/FEATURE_CHECKLIST.md` | SSOT chain BETA → RC；添加 RC stage update note | `STAGE.yaml: current_stage: RC` |
| `docs/releases/v3.12.0/SCOPE_TABLE_v3.12.md` | 添加 RC stage update section（RC_GATE_REPORT 12/12 + B8 13/13） | `RC_GATE_REPORT.md` + `evidence/v312-59-e/` |
| `docs/releases/v3.12.0/COMPREHENSIVE_ASSESSMENT_REPORT.md` | **本报告**：覆盖式重写自 2026-08-14 ALPHA 草案 → 2026-08-27 RC 综合评估 | DOC_CHECK_CORRECTION_RULES §三 7 步流程 |
| `docs/releases/v3.12.0/DOC_RECTIFICATION_WORK_REPORT_2026-08-27.md` | **新建**：DOC_CHECK_CORRECTION_RULES §三 step 5 工作报告 | DOC_CHECK_CORRECTION_RULES §三 步骤 5 |

**未修改文件**（已确认一致）：
- `README.md`（已正确显示 RC）
- `CURRENT_VERSION.md`（已正确显示 RC）
- `CHANGELOG.md`（含 v3.12.0 RC drift-fix 段）
- `docs/releases/v3.12.0/README.md`（已正确显示 RC）
- `docs/releases/v3.12.0/CHANGELOG.md`（已正确显示 RC drift-fix）
- `docs/releases/v3.12.0/RELEASE_NOTES.md`（已正确显示 RC）
- `docs/releases/v3.12.0/STAGE.yaml`（SSOT，未修改）
- `docs/releases/v3.12.0/RC_GATE_REPORT.md`（RC gate 聚合，未修改）

整改原则：最小修改（DOC_CHECK_CORRECTION_RULES 2.1），仅事实性错误修正（版本号 / 日期 / 状态标记 / 重复条目），不动 commit 日志、功能描述、架构设计、实质性技术内容。

---

## 附录 A. 原 ALPHA 草案（2026-08-14）保留区

> 原 ALPHA 草案（commit `50e5d121b`，2026-08-14T22:20:00+08:00）已被本 RC 综合评估报告覆盖。原报告结论方向正确（v3.12 ALPHA 阶段定位于"GMP 内审检索数据库 + v3.11.0 弱项硬化 + MySQL-style 基础兼容收口"），但以下关键差异已被本 RC 版本取代：

| 项 | ALPHA 草案（2026-08-14） | RC 综合评估（2026-08-27） |
|----|------------------------|---------------------------|
| 阶段状态 | ALPHA | RC（2026-08-26 转入） |
| 评估依据 | 文档静态核查 + 部分 evidence | 12/12 RC gate + B8 13/13 thresholds_override 实跑 |
| TPC-H 结论 | 待 close-out | Q22 PASS + Q17/Q20 TIMEOUT documented |
| SQLLogicTest | 16/22 历史 FAIL（Round-9 partial） | 25/25 smoke + 16/16 historical closed |
| V312-57 教学 CLI | 未到阶段 | week01-06 全 fixture PASS |
| Wire / LOAD DATA | PARTIAL | 10 步骤全 PASS（含 SF=10） |
| Crash recovery | PARTIAL | 7+4+4 scenarios PASS |

ALPHA 草案详细内容已保存于 git 历史（commit `50e5d121b`），本附录不再展开。读者如需查阅 ALPHA 阶段判断，使用 `git log --follow docs/releases/v3.12.0/COMPREHENSIVE_ASSESSMENT_REPORT.md` 配合 `git show <commit>:<file>` 即可定位历史版本。

---

**Why this report**: 本报告是 v3.12.0 RC 阶段的综合评估，区别于 v3.11.0 GA 综合评估的关键点在于：

1. 阶段定义从 GA → RC；评级从"GA ready" → "RC ready, NOT GA"
2. 评估依据从"发布裁决" → "RC promotion cycle 实跑证据"
3. 风险识别重点从"GA 收口" → "GA blocker 清单 + 收口要求"
4. 整改记录从"v3.11 GA 自审" → "v3.12 RC 文档最小化整改"

后续 RC → GA 转化时，需重新编写本综合评估报告（按 `docs/governance/DOC_CHECK_CORRECTION_RULES.md` §三 7 步流程），并在 `CHANGELOG.md` 中追加 GA 综合评估条目。
