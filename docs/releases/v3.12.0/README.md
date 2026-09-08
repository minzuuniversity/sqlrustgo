# SQLRustGo v3.12.0

> **元数据**: generated_by=claude-macmini, generated_at=2026-09-08, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, head=`34d8adc56c`, policy=Anti-Fabrication-Policy-v1.0 + ADR-001 + ADR-014
> **状态**: **GA**（2026-09-08，72/72 gate PASS）
> **SSOT**: `STAGE.yaml` 是版本阶段的事实来源（Source of Truth）

---

## 0. 当前状态（2026-09-08）

### GA 门禁状态

| Gate | 状态 |
|------|------|
| BETA | 40/40 ✅ PASS |
| RC | 11/11 ✅ PASS |
| GA | 8/8 ✅ PASS |
| thresholds_override | 13/13 ✅ PASS |
| **总计** | **72/72 ✅ PASS** |

- commit: `34d8adc56cc351db19182fb852056d4d7483fa00`
- generated_at: `2026-09-08T03:48:45Z`
- mode: `full`
- blockers: 0

### 版本定位

v3.12.0 是 SQLRustGo 的 **GMP 合规性内审检索系统** 数据库底座。允许的产品范围：

- SQLRustGo 管理的 GMP 文档、chunk、版本、审计记录、证据关系和检索元数据的关系存储
- 面向 GMP/RAG 工作负载的内部向量检索和混合检索
- 用于证据导航的 SQL-backed 图投影
- 仅在验证的兼容性边界内提供 MySQL 风格和 sqlite-like 入口

**禁止声明**：
- ❌ 通用向量数据库
- ❌ 通用图数据库
- ❌ 广泛的 MySQL/SQLite 替代品

### 已知限制 — GA-claim-caveat

| Issue | 区域 | 限制范围 |
|-------|------|----------|
| #4846 | 执行器/类型语义 | `CHAR(n)` 字节填充主键点查排除；`VARCHAR` 不受影响 |
| #4847 | 事务语义 | 显式 `BEGIN`/`COMMIT`/`ROLLBACK` 排除；GMP 产品使用单语句批处理模式 |
| #4848 | 存储/DDL | `ALTER TABLE ... RENAME COLUMN` 排除；`ADD COLUMN`/`DROP COLUMN` 不受影响 |

详见 `CLAIM_DOWNGRADE_MANIFEST.md` §9.6。

---

## 1. 功能完整性矩阵

### 1.1 主路径能力（v3.11 GA vs v3.12 GA）

| # | 能力 | v3.11 GA | v3.12 GA | 证据 |
|---|------|---------|---------|------|
| 1 | TPC-H SF=1 in-process 22/22 | PASS | **PASS** | Q17 61.6s PASS |
| 2 | TPC-H SF=1 wire round-trip 22/22 | PASS | **PASS** | cross-engine matrix |
| 3 | TPC-H SF=10 bulk load | 部分 | **PASS** | LOAD DATA SF=10 PASS |
| 4 | GMP 文档/Chunk/Embedding/Audit schema | 无 | **PASS** | RC1 wrapper |
| 5 | GMP hybrid retrieval | 无 | **PASS** | RC2 wrapper |
| 6 | SQL-backed graph projection | 无 | **PASS** | depth-limited paths + neighbors |
| 7 | 审计 hash-chain tamper fail-closed | 无 | **PASS** | RC1/RC4 |
| 8 | SQLLogicTest smoke 25/25 | 无 | **PASS** | smoke-report.md |
| 9 | V312-57 sqlite3-like CLI | 无 | **PASS** | RC9 + RC10 |
| 10 | MySQL wire 协议 typed wrappers | 部分 | **PASS** | RC7 wrapper |
| 11 | MySQL TLS 1.3 / compression | 部分 | **PASS** | V312-13-REPORT.md |
| 12 | MySQL prepared statement params | 部分 | **PASS** | V312-13-REPORT.md |
| 13 | LOAD DATA SF=1 / SF=10 | 部分 | **PASS** | V312-13-REPORT.md |
| 14 | Backup / restore (GMP preserved) | 部分 | **PASS** | RC3 wrapper |
| 15 | Crash recovery 7+4+4 | 部分 | **PASS** | RC8 wrapper |
| 16 | RBAC + 安全扫描 | 部分 | **PASS** | RC4 wrapper |
| 17 | 168h SOAK | PASS | **PARTIAL** | GA2 mixed demo PASS；scaffold 就绪 |
| 18 | per-crate 覆盖率 ≥80% | 部分 | **PARTIAL** | 分层口径 |
| 19 | RAG inverted index + rerank | 部分 | **PASS** | RC4 + RC7 |
| 20 | 教学 REPL + 内审检索 demo | 部分 | **PASS** | V312-57 week01-06 |
| 21 | 文档治理 0 overclaim | 部分 | **PASS** | CLAIM_DOWNGRADE_MANIFEST |

**总计**：19/21 PASS，2/21 PARTIAL

### 1.2 TPC-H SF=1 Cross-Engine 对比

| 引擎 | 覆盖数 | 说明 |
|------|--------|------|
| PostgreSQL | 22/22 | 参考 Oracle |
| SQLite | 22/22 | 参考 Oracle |
| MySQL | 18/22 | Q2/Q11/Q12/Q17 deferred（v3.13） |
| **SQLRustGo** | **22/22** | **Q17 61.6s PASS（was TIMEOUT 1042s）** |

### 1.3 SQLLogicTest 状态

| 类别 | 状态 | 说明 |
|------|------|------|
| smoke | **25/25 PASS** | `smoke-report.md` |
| curated selected | **16/21 PASS** | 5 EXCLUDED（issue-linked） |
| historical exclusions | **16/16 closed** | v3.12.0 内关闭 |
| full SQLite corpus | 待 v3.13 | RC/GA expansion |

---

## 2. 稳定性与性能评估

### 2.1 TPC-H SF=1

| 维度 | 结论 | 证据 |
|------|------|------|
| In-process 22/22 执行 | **PASS** | SUMMARY.json |
| Cross-engine 4 engine | **sqlrustgo 22/22** | 4 engine × 22 query |
| Q17 SF=1 | **PASS 61.6s** | PR #4550 (commit `640d672bf8`) |
| Cell-level 匹配 | **22/22** | Q17_SF1_CELLDIFF.json |

### 2.2 SOAK

| 阶段 | 状态 | 证据 |
|------|------|------|
| v3.11 GA 168h SOAK | PASS（343h37m） | SOAK_168H_REPORT.md |
| v3.12 GA mixed demo | PASS | GA2_MIXED_SOAK_DEMO_REPORT.md |
| 168h scaffold | 就绪 | tests/soak/v312_mixed_soak.rs |

### 2.3 LOAD DATA / Bulk

| 测试 | 状态 | 证据 |
|------|------|------|
| SF=0.0001 smoke | PASS | V312-13-REPORT.md step 6.5 |
| SF=1 | PASS | step 7 |
| SF=10 | PASS | step 8 |
| TLS handshake | PASS | step 9 |
| Compression | PASS | step 10 |

### 2.4 Crash Recovery

| 场景 | 状态 | 证据 |
|------|------|------|
| 7+4+4 scenarios | PASS | V312-14-CRASH-RECOVERY-RECHECK.md |

---

## 3. 安全与合规

| 项 | 状态 | 证据 |
|----|------|------|
| RBAC role-based access | PASS | GA-4 wrapper |
| Audit hash-chain tamper fail-closed | PASS | RC1/RC4 |
| 文档 claim 清理 | PASS | 4 ALLOWED / 14 DISALLOWED / 0 OVERCLAIM |
| 安全扫描 | PASS | GA3_SECURITY_SCAN_REPORT.md |
| Secret 扫描 | PASS | secret_scan_v312.txt |
| Plaintext password 扫描 | PASS | plaintext_pw_scan_v312.txt |

---

## 4. 目录结构

```
v3.12.0/
├── README.md                              # 本文件 - 版本索引
├── CHANGELOG.md                           # 变更日志
├── RELEASE_NOTES.md                       # 发布说明
├── STAGE.yaml                             # 阶段 SSOT
├── GA_GATE_REPORT.md                      # GA 门禁报告
├── RC_GATE_REPORT.md                      # RC 门禁报告
├── COMPREHENSIVE_TEST_FRAMEWORK_AND_COVERAGE_BASELINE.md  # 综合测试框架
├── CLAIM_DOWNGRADE_MANIFEST.md            # Claim 降级清单
├── TEST_PLAN.md                           # 测试计划
├── SCOPE_TABLE_v3.12.md                   # 范围表
│
├── evidence/                              # 证据目录
│   ├── v312-59/                          # v3.12.0-59 门禁证据
│   ├── tpch/                             # TPC-H 正确性证据
│   ├── sqllogictest/                     # SQLLogicTest 证据
│   └── gmp_compliance/                   # GMP 合规性证据
│
├── perf/                                 # 性能基线
├── sqllogictest-baseline/                # SQLLogicTest 基线
├── sql-feature-corpus/                   # SQL 功能语料库
└── issues/                               # Issue 追踪
```

---

## 5. 阶段演进

| 阶段 | 日期 | 状态 | 关键里程碑 |
|------|------|------|-----------|
| ALPHA | 2026-08-19 | ✅ PASS | GMP schema、文档摄取、混合检索、图投影基础 |
| BETA | 2026-08-19 | ✅ PASS | 40/40 gate PASS |
| RC | 2026-08-26 | ✅ PASS | 11/11 RC gate PASS |
| GA | 2026-09-08 | ✅ PASS | 72/72 gate PASS |

---

## 6. GA 门禁清单

- ✅ 新鲜 `--full` gate verdict PASS at HEAD `34d8adc56c`（72/72, 0 blockers）
- ✅ `STAGE.yaml` `gate_snapshot` 已更新
- ✅ `CLAIM_DOWNGRADE_MANIFEST.md` §9.6 记录 3 个 GA-claim-caveat 项目
- ✅ `README.md` "已知限制" 章节列出 3 个边界声明
- ⏳ 按 STAGE_CONFIG RC_to_GA trigger cut `v3.12.0` + `v3.12.0-ga` tags

---

## 7. 关键文档索引

| 文档 | 用途 |
|------|------|
| `STAGE.yaml` | 阶段 SSOT 和升级要求 |
| `GA_GATE_REPORT.md` | 当前 GA verdict map 和证据边界 |
| `RELEASE_CHECKLIST.md` | RC→GA 操作清单 |
| `RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md` | RC-GA issue 分类和门禁计划 |
| `CLAIM_DOWNGRADE_MANIFEST.md` | Release-claim 排除和关闭台账 |
| `TEST_PLAN.md` | 测试策略和门禁期望 |
| `COMPREHENSIVE_TEST_FRAMEWORK_AND_COVERAGE_BASELINE.md` | 分层覆盖率/测试框架 |
| `RELEASE_NOTES.md` | 发布说明 |
| `CHANGELOG.md` | 变更日志 |
| `COMPREHENSIVE_ASSESSMENT_REPORT.md` | 综合评估报告 |

---

## 8. 变更历史

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-08-26 | v3.12.0-rc1 | BETA → RC 推进完成 |
| 2026-09-08 | v3.12.0 | GA promotion authorized，72/72 gate PASS |

---

*本文档由 claude-macmini 维护*
