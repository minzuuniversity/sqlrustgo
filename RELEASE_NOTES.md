# SQLRustGo 发行说明

> **当前活跃版本**: **v3.12.0** (BETA, 2026-08-19 转入)
> **维护人**: yinglichina8848
> **更新日期**: 2026-08-20

本文件是 **所有发行版本的索引页**。每个版本的详细 release notes 见
`docs/releases/<version>/RELEASE_NOTES.md`。

---

## 最新版本

| 版本 | 状态 | 发布日期 | 一句话总结 | 详细 |
|------|------|---------|-----------|------|
| **v3.12.0** | **BETA** | 2026-08-19 | GMP 内审检索数据库 + V312-57 sqlite3-like 教学 CLI (14/14 gate PASS, Beta Gate 0 BLOCKERS) | [`docs/releases/v3.12.0/RELEASE_NOTES.md`](docs/releases/v3.12.0/RELEASE_NOTES.md) |
| v3.11.0 | GA | 2026-08-09 | TPC-H SF=1 22/22 PASS + 343h SOAK + 23 V311 tasks | [`docs/releases/v3.11.0/RELEASE_NOTES.md`](docs/releases/v3.11.0/RELEASE_NOTES.md) |
| v3.8.0 | GA | 2026-06-08 | Long Convergence Release (WAL + MVCC 强约束) | [`docs/releases/v3.8.0/RELEASE_NOTES.md`](docs/releases/v3.8.0/RELEASE_NOTES.md) |
| v3.7.0 | GA | 2026-05-30 | 稳定性增强（覆盖率 87.36%）| [`docs/releases/v3.7.0/RELEASE_NOTES.md`](docs/releases/v3.7.0/RELEASE_NOTES.md) |
| v3.6.0 | GA | 2026-05-30 | CBO + TPC-H 22/22 | [`docs/releases/v3.6.0/RELEASE_NOTES.md`](docs/releases/v3.6.0/RELEASE_NOTES.md) |
| v3.5.0 | GA | 2026-05-28 | AI Native GMP Platform | [`docs/releases/v3.5.0/RELEASE_NOTES.md`](docs/releases/v3.5.0/RELEASE_NOTES.md) |
| v3.4.0 | GA | 2026-05-24 | GMP Management Suite | [`docs/releases/v3.4.0/RELEASE_NOTES.md`](docs/releases/v3.4.0/RELEASE_NOTES.md) |
| v3.3.0 | GA | 2026-05-20 | Industrial Trust Platform | [`docs/releases/v3.3.0/RELEASE_NOTES.md`](docs/releases/v3.3.0/RELEASE_NOTES.md) |
| v3.2.0 | GA | 2026-05-18 | Trust Convergence | [`docs/releases/v3.2.0/RELEASE_NOTES.md`](docs/releases/v3.2.0/RELEASE_NOTES.md) |

---

## v3.9.0 速览 (GA)

### 三大主题
1. **TPC-H 22/22** — SF=0.1 in-process + wire-protocol 双通过，21/22 cell-level 匹配 SQLite（Q22 为已知 SQL 标准差异）
2. **Q13 子查询修正** — `NOT IN (subquery_with_LIKE)` 三值逻辑正确（was 60/60 → now 11/11 excluded）
3. **Q9 6.7x 加速** — 600ms → 90ms via hash-join pre-filter pushdown

### 关键指标
| 指标 | 结果 |
|------|------|
| TPC-H SF=0.1 | 22/22 PASS，耗时 -92%（30s→2.3s）|
| TPC-H SF=1 | 6/10 PASS（4 个 parser 限制报错，12 个未实现）|
| Cell-level 匹配 | 21/22（Q22 SQL 标准差异）|
| 72h SOAK | ✅ 119h57m，0 错误，0 重连 |
| 168h SOAK | ✅ PASS |
| G13 Deadlock | ✅ 已修复（parking_lot RwLock）|

### 已知限制（GA 条件通过）
| 项目 | 状态 | 说明 |
|------|------|------|
| 覆盖率 | ⚠️ ~67% < 85% | 条件通过；目标 v3.10.0 GA ≥80% |
| TPC-H H/22 |
| 168h SOAK | ✅ PASS | — |

详见 [v3.9.0 GA 发行说明](docs/releases/v3.9.0/ga/GA_RELEASE_NOTES.md) 与 [GA 门禁报告](docs/releases/v3.9.0/ga/GA_GATE_REPORT.md)。

### v3.9.0 GA 里程碑
| 里程碑 | 状态 |
|--------|------|
| TPC-H 22/22 in-process | ✅ |
| TPC-H 22/22 wire round-trip | ✅ |
| Cell-level MATCH 21/22 (vs SQLite) | ✅ |
| Q9 6.7x 加速（600ms → 90ms）| ✅ |
| Q13 子查询修正 | ✅ |
| 72h SOAK（0 错误，0 重连）| ✅ |
| G13 deadlock 修复（parking_lot RwLock）| ✅ |
| execution_engine.rs 2630 → 1471 行 | ✅ |
| Statement cache（1.7x 热路径加速）| ✅ |
| SCRAM-SHA-256 加固 | ✅ |
| TLS 1.3 默认启用 | ✅ |
| 168h SOAK | ✅ PASS |

### v3.9.0 已知限制（GA 条件通过）
| 项目 | 状态 | 说明 |
|------|------|------|
| 覆盖率均值 | ⚠️ ~67% < 85% | 条件通过；目标 v3.10.0 GA ≥80% per crate |
| TPC-H H/22 |

### 升级说明
v3.9.0 是 v3.8.0 的 **二进制互换升级**，无需数据迁移。
详见 [v3.9.0 升级指南](docs/releases/v3.9.0/MIGRATION_GUIDE.md)。

### 关键文档
- [v3.9.0 GA 发行说明](docs/releases/v3.9.0/ga/GA_RELEASE_NOTES.md)
- [GA 门禁报告](docs/releases/v3.9.0/ga/GA_GATE_REPORT.md)
- [覆盖率缺口说明](docs/releases/v3.9.0/ga/COVERAGE_GAP_RATIONALE.md)
- [TPC-H SF=1 部分结果说明](docs/releases/v3.9.0/ga/TPC-H_PARTIAL_RESULT.md)
- [v3.9.0 性能报告](docs/releases/v3.9.0/ga/PERFORMANCE_REPORT.md)
- [v3.9.0 安全审计](docs/releases/v3.9.0/ga/SECURITY_AUDIT.md)

## v3.8.0 速览 (GA)

**Long Convergence Release** — 长期收敛版本。WAL + MVCC Mandatory 路径统一。

### 关键变更
- **Retired legacy binaries**: `sqlrustgo` / `sqlrustgo-sql-cli` / `sqlrustgo-bench` 等退役
- **Canonical entry**: `sqlrustgo-mysql-server` (subcommands: serve / exec / repl / bench / gmp / diag / backup / restore)
- **Embedded test harness keystone**: `start_ephemeral` + `MySqlTestClient` 启用 raw wire-protocol 测试
- **Gate scripts updated**: `check_alpha_v380.sh` 增 `A1_BIN_COUNT` + `A2_EPHEMERAL_SMOKE`

详见 [`docs/releases/v3.8.0/RELEASE_NOTES.md`](docs/releases/v3.8.0/RELEASE_NOTES.md)。

---

## v3.7.0 速览 (GA)

**Stability Enhancement** — 87.36% 覆盖率 + 39 项单元测试新增。

详见 [`docs/releases/v3.7.0/RELEASE_NOTES.md`](docs/releases/v3.7.0/RELEASE_NOTES.md)。

---

## v3.6.0 速览 (GA)

**Performance Optimization** — CBO 成本优化器 + TPC-H 22/22 PASS。

详见 [`docs/releases/v3.6.0/RELEASE_NOTES.md`](docs/releases/v3.6.0/RELEASE_NOTES.md)。

---

## v3.5.0 速览 (GA)

**AI Native GMP Platform** — Deviation Investigator / Compliance Judge / Device Predictor / Report Generator。

详见 [`docs/releases/v3.5.0/RELEASE_NOTES.md`](docs/releases/v3.5.0/RELEASE_NOTES.md)。

---

## 历史版本

| 版本 | 状态 | 一句话总结 | 详细 |
|------|------|-----------|------|
| v3.4.0 | GA | GMP Management Suite (EBR + Electronic Signature + Audit) | [`docs/releases/v3.4.0/RELEASE_NOTES.md`](docs/releases/v3.4.0/RELEASE_NOTES.md) |
| v3.3.0 | GA | Industrial Trust Platform | [`docs/releases/v3.3.0/RELEASE_NOTES.md`](docs/releases/v3.3.0/RELEASE_NOTES.md) |
| v3.2.0 | GA | Trust Convergence | [`docs/releases/v3.2.0/RELEASE_NOTES.md`](docs/releases/v3.2.0/RELEASE_NOTES.md) |
| v3.0.0 | GA | SQL-92 完整支持 (基础) | [`docs/releases/v3.0.0/RELEASE_NOTES.md`](docs/releases/v3.0.0/RELEASE_NOTES.md) |
| v2.9.0 | GA | 性能 + 分布式基础 | [`docs/releases/v2.9.0/RELEASE_NOTES.md`](docs/releases/v2.9.0/RELEASE_NOTES.md) |
| v2.8.0 | GA | 生产化 + 分布式 + 安全 | [`docs/releases/v2.8.0/RELEASE_NOTES.md`](docs/releases/v2.8.0/RELEASE_NOTES.md) |
| v2.7.0 | GA | 企业级韧性 (WAL 崩溃恢复) | [`docs/releases/v2.7.0/RELEASE_NOTES.md`](docs/releases/v2.7.0/RELEASE_NOTES.md) |
| v2.6.0 | GA | SQL-92 完整支持 | [`docs/releases/v2.6.0/RELEASE_NOTES.md`](docs/releases/v2.6.0/RELEASE_NOTES.md) |
| v1.0.0 | GA | 首个正式版本 | (早期版本) |

> **v1.0.0**: 首个正式版本 (`RELEASE_NOTES.md` 旧版位于 git 历史)。支持 SQL-92 子集 + 基础存储引擎 + 索引 + 事务 + 网络协议。

---

## 发行流程 (Release Pipeline)

每个 GA 版本经历：
1. **Alpha**: 功能冻结, 单元测试 + 集成测试 PASS, 文档一致性
2. **Beta**: 边界测试, 回归测试, 性能基线
3. **RC**: 生产就绪检查, GA gate (D1-D9 for v3.8, G1-G16 for v3.9)
4. **GA**: 所有 gate PASS, release notes + changelog + roadmap 完整

详细 SOP: [`docs/RELEASE_NORMALIZATION.md`](docs/RELEASE_NORMALIZATION.md), [`RELEASE_GOVERNANCE.md`](RELEASE_GOVERNANCE.md)

---

## 维护说明

- 本索引页与 [CHANGELOG.md](CHANGELOG.md) 互补：CHANGELOG 强调技术变更按时间线, RELEASE_NOTES 强调功能分类
- 版本路线图见 [ROADMAP.md](ROADMAP.md)
- 治理规范见 [BRANCH_GOVERNANCE.md](BRANCH_GOVERNANCE.md), [RELEASE_GOVERNANCE.md](RELEASE_GOVERNANCE.md)
- 任何版本发布前需更新对应版本的 CHANGELOG + ROADMAP + 本索引页
