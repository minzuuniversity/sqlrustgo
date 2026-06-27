# SQLRustGo 发行说明 (Release Notes)

> **当前活跃版本**: **v3.9.0** (RC7, 2026-06-17, GA 目标 2026-12-15)
> **Latest GA**: **v3.8.0** (2026-06-04)
> **维护人**: yinglichina8848
> **更新日期**: 2026-06-17

本文件是 **所有发行版本的索引页**。每个版本的详细 release notes 见
`docs/releases/<version>/RELEASE_NOTES.md`。

---

## 最新版本

| 版本 | 状态 | 发布日期 | 一句话总结 | 详细 |
|------|------|---------|-----------|------|
| **v3.9.0** | RC7 | 2026-12-15 (GA 目标) | Production-readiness：TPC-H 22/22 + Q8 165,000× + Q13 subquery fix | [`docs/releases/v3.9.0/RELEASE_NOTES.md`](docs/releases/v3.9.0/RELEASE_NOTES.md) |
| v3.8.0 | GA | 2026-06-04 | Long Convergence Release (WAL + MVCC 强约束) | [`docs/releases/v3.8.0/RELEASE_NOTES.md`](docs/releases/v3.8.0/RELEASE_NOTES.md) |
| v3.7.0 | GA | 2026-05-30 | Stability enhancement (87.36% 覆盖率) | [`docs/releases/v3.7.0/RELEASE_NOTES.md`](docs/releases/v3.7.0/RELEASE_NOTES.md) |
| v3.6.0 | GA | 2026-05-30 | CBO + TPC-H 22/22 | [`docs/releases/v3.6.0/RELEASE_NOTES.md`](docs/releases/v3.6.0/RELEASE_NOTES.md) |
| v3.5.0 | GA | 2026-05-28 | AI Native GMP Platform | [`docs/releases/v3.5.0/RELEASE_NOTES.md`](docs/releases/v3.5.0/RELEASE_NOTES.md) |
| v3.4.0 | GA | 2026-05-24 | GMP Management Suite | [`docs/releases/v3.4.0/RELEASE_NOTES.md`](docs/releases/v3.4.0/RELEASE_NOTES.md) |
| v3.3.0 | GA | 2026-05-20 | Industrial Trust Platform | [`docs/releases/v3.3.0/RELEASE_NOTES.md`](docs/releases/v3.3.0/RELEASE_NOTES.md) |
| v3.2.0 | GA | 2026-05-18 | Trust Convergence | [`docs/releases/v3.2.0/RELEASE_NOTES.md`](docs/releases/v3.2.0/RELEASE_NOTES.md) |

---

## v3.9.0 速览 (RC7)

### 三大主题
1. **TPC-H 22/22** — in-process + wire-protocol 双通过，21/22 cell-level match SQLite (Q22 有 SQL 标准差异)
2. **Q13 subquery fix** — `NOT IN (subquery_with_LIKE)` 三值逻辑正确 (was 60/60 → now 11/11 excluded)
3. **Q9 6x faster** — 600ms → 90ms via hash-join pre-filter pushdown

### Sprint 8 增量 (2026-06-17)
- **Q8 165,000× faster** — Track A: 33,000ms → 0.18ms via `extract_comma_join_keys` + `JoinKey::All` hash-join fallback
- **ADR-006 V5/V6/V8/V2 治理** — 5/5 meta-gates (P11/P12/P13/P14/P15) 全部 PASS
- **`sqlrustgo-mysql-server soak` 子命令** — 真实 wall-clock 长期浸泡 binary, 24h/72h/168h infra READY
- **26 long-stability 测试分析** — 全部需 Z6G4 验证

### On-disk Format
**v3.8.0 → v3.9.0 零数据迁移** (binary-swap upgrade)。

### Known Gaps (Open)
- 4 EAGAIN-failing integration tests on macOS debug build ([Issue #3307](http://192.168.0.250:3000/openclaw/sqlrustgo/issues/3307))
- 24h/72h/168h real wall-clock soak pending Z6G4 hardware
- v3.10 wired-soak DDL + wire protocol repair ([Issue #3302](http://192.168.0.250:3000/openclaw/sqlrustgo/issues/3302))

### 迁移指南
详见 [`docs/releases/v3.9.0/MIGRATION_GUIDE.md`](docs/releases/v3.9.0/MIGRATION_GUIDE.md)。

### 关键文档
- [Comprehensive Assessment v2.0](docs/releases/v3.9.0/V390_COMPREHENSIVE_ASSESSMENT.md)
- [Evidence Status](docs/releases/v3.9.0/EVIDENCE_STATUS.md)
- [GA Gate Report](docs/releases/v3.9.0/GA_GATE_REPORT.md)
- [GA Readiness Final (2026-06-19)](docs/releases/v3.9.0/GA_READINESS_FINAL_2026-06-19.md)
- [Changelog](docs/releases/v3.9.0/CHANGELOG.md)
- [Roadmap](docs/releases/v3.9.0/ROADMAP.md)
- [Evaluation Report](docs/releases/v3.9.0/EVALUATION_REPORT.md)
- [Feature Matrix](docs/releases/v3.9.0/FEATURE_MATRIX.md)
- [Q8 Performance Analysis](docs/releases/v3.9.0/Q8_PERF_ANALYSIS.md)
- [TPC-H E2E Testing](docs/releases/v3.9.0/TPCH_E2E_TESTING.md)

---

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
