# SQLRustGo v4.0.0 升级指南

> **适用升级路径**: v3.12.x → v4.0.0
> **状态**: GA — General Availability (CONDITIONAL PASS)
> **配套文档**: [RELEASE_NOTES.md](./RELEASE_NOTES.md) / [CLAIM_DOWNGRADE_MANIFEST.md](./CLAIM_DOWNGRADE_MANIFEST.md) / [GA_GATE_REPORT.md](./GA_GATE_REPORT.md)
> **说明**: 本中文主文用于当前正式阅读；英文原文保留在附录。升级指南描述推荐流程，**不等同**升级/回滚测试已经全部通过（详细覆盖率与边界见 GA_GATE_REPORT + CLAIM_DOWNGRADE_MANIFEST）。

---

## 1. 升级前检查（preflight）

### 1.1 数据 / 元数据

- 备份当前数据库文件、WAL、配置和 release metadata。
- 记录当前版本、commit hash、数据目录 hash 和关键表 row count。
- 记录当前所有 active storage backend 配置（FileStorage / MvccStorage / WalStorage 启用情况）。

### 1.2 依赖与外部约束

- 确认 OS / glibc 与 v4.0.0 binary 兼容（与 v3.12.0 同基线）。
- 确认 MySQL 客户端版本（5.7 / 8.0）未受影响（v4.0.0 无 wire-protocol breaking change）。
- 确认 GMP-Platform 消费方使用 PR #207 状态（如未消费 V400-10，可忽略）。

### 1.3 已知风险边界

按 `CLAIM_DOWNGRADE_MANIFEST.md` §3：
- G3 coverage 78.28% 平均 < 80% GA 阈值（CONDITIONAL PASS）— `sqlrustgo-parser` 74.32% 和 `sqlrustgo-mysql-server` 75.41% 是主缺口
- 168h SOAK 仅完成 5min pre-flight（RSS peak 636 MB / 118585 queries / 0 errors）；168h 实跑 defer 至 V400-09 follow-up
- 5/8 WP 推 v4.0.1（WP-C / WP-D / WP-F / WP-G / WP-B 部分）
- V400-09 168h SOAK、V400-10 GMP-Platform consumer、V400-08 cost model 在 v4.0.0 切出时仍处 🟡 状态

---

## 2. 升级流程（推荐）

### 2.1 维护窗口

1. 停止写入或将流量切换到维护窗口。
2. 完成完整备份（数据 + WAL + 配置），并做一次 restore rehearsal。
3. 在灰度环境用 v4.0.0 binary 启动并执行 §2.2 一致性 check。

### 2.2 一致性 check

1. 数据目录 hash 对比：升级前后数据目录 SHA256 比对（v4.0.0 默认存储路径与 v3.12.0 一致）。
2. WAL replay：v4.0.0 启动时自动 replay WAL；新增 6 种 vector WAL entry types（向后兼容 v3.x WAL 格式）。
3. 核心业务 SQL regression：TPC-H SF=1 22/22（继承自 v3.12.0 GA）、自定义核心 SQL subset、row count + SHA256 比对。
4. Schema/权限回归：现有 schema / ACL / role / audit chain 不应被破坏（v4.0.0 不引入 schema migration）。

### 2.3 流量切换

1. 灰度 5% 流量，观察 RSS（应 < 1.5 GB cap，预估 ~636 MB peak）。
2. 验证 MVCC GC 工作：RSS 振幅应在 311–636 MB 区间（参考 `SOAK_BASELINE_5MIN_2026-09-19.md`）。
3. 启用 V400 multi-model opt-in（仅对启用 V400-02 / V400-03 的库；默认行为不变）。
4. 扩大流量并持续观察 24–72 小时。

---

## 3. 回滚要求

### 3.1 回滚前置

- 必须有升级前的完整数据目录快照（包含 v3.12.x 格式 WAL）。
- 必须有 v3.12.x binary / 配置可立即切换。
- 回滚必须验证 row count、hash、关键查询结果、ACL 策略一致性。

### 3.2 v4.0.0 → v3.12.x 回滚

- v4.0.0 WAL 兼容 v3.x replay（无格式破坏性）— 回滚可读 v4.0.0 WAL 但 v3.x 不识别 v4.0.0 写入的新 vector WAL 类型。
- **如升级期间未触发 vector WAL 写入**（默认场景），回滚到 v3.12.x 是干净的。
- **如升级期间已写入 vector WAL entry**，回滚到 v3.12.x 会丢失这部分 WAL replay 状态——必须先 dump + truncate WAL，或保留 v4.0.0 binary 直至数据归档完成。

### 3.3 不可回滚场景

- 已启用 V400-05/06/07 PRODUCTION claim（跨模型事务、BackupCoordinator、AuditChain）后回滚 v3.12.x — 这些机制的状态在 v3.12.x 中不存在，需保留 v4.0.0 binary 或迁移至 v4.0.1。

---

## 4. 已知兼容性问题

### 4.1 存储 / 性能
- 默认 storage backend 由 v3.12.0 的 MvccStorage<FileStorage> 沿用至 v4.0.0（无切换成本）
- WAL group commit 启用后，写入路径 fsync 频率变化；TPS 提升约 30%（batch:100 模式），单条写入尾延迟不变
- RSS 上限：从 v3.12.0 的 ~1.5 GB cap 压至 ~636 MB（MVCC GC 调优）

### 4.2 SQL 语义
- 无 DDL breaking change
- 无 DML 语义变化
- V400-01 / V400-04 引入的新语法（vector SQL / Cypher MATCH / GRAPH MATCH）仅当显式启用对应 storage 时生效

### 4.3 协议 / 客户端
- MySQL wire-protocol 无变化
- Admin / replication / audit log 客户端无需调整

---

## 5. 升级 checklist（PR description 模板用）

```markdown
## v3.12.x → v4.0.0 升级 PR

### 升级前
- [ ] 数据 + WAL + 配置 完整备份完成
- [ ] 备份 SHA256 验证通过
- [ ] 灰度环境 v4.0.0 binary 启动 OK
- [ ] 核心 SQL regression 通过

### 升级中
- [ ] 维护窗口 + 流量切换
- [ ] WAL replay 验证（vector WAL 类型向后兼容）
- [ ] Schema / ACL / audit chain 完整性
- [ ] RSS 监测 ≤ 1.5 GB（实际 ~636 MB peak）

### 升级后
- [ ] 24h 流量观察 RSS 振幅 311–636 MB
- [ ] 关键业务查询 row count + SHA256 比对通过
- [ ] V400 multi-model opt-in 评估（如需要）
- [ ] 回滚方案备好（binary + 数据快照 + WAL 策略）

### 已知边界（需在 PR 描述同步）
- [ ] G3 coverage CONDITIONAL PASS（78.28% < 80%）
- [ ] 168h SOAK defer（5min pre-flight PASS）
- [ ] WP-C/D/F/G / WP-B 部分 defer-to-v4.0.1
- [ ] V400-08 / V400-10 在 v4.0.0 切出时 🟡 状态
```

---

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义；当前正式阅读与执行口径以上方中文正文为准。

# v3.12.0 → v4.0.0 Upgrade Guide

## Preflight

- Backup data files, WAL, config, and release metadata.
- Record current version, commit, data dir hash, and critical table row counts.
- Record active storage backend configuration (FileStorage / MvccStorage / WalStorage).

## Upgrade Procedure

1. Stop writes or switch to maintenance window.
2. Full backup + restore rehearsal.
3. Deploy v4.0.0 binary/config.
4. Run schema/data consistency checks (data dir SHA256, WAL replay, row count, TPC-H SF=1 22/22).
5. Observe SOAK/health metrics, then ramp traffic.

## Rollback

- Requires pre-upgrade data dir snapshot and v3.12.x binary available.
- v4.0.0 WAL is backward-compatible with v3.x replay (no breaking WAL format).
- **If vector WAL entries were written during the upgrade window**: rollback to v3.12.x loses that WAL replay state. Either dump+truncate WAL, or retain v4.0.0 binary until data is archived.
- **Once V400-05/06/07 PRODUCTION claims are enabled**: rollback to v3.12.x loses these mechanisms. Stay on v4.0.0 or migrate to v4.0.1.

## Known Boundaries

- G3 coverage 78.28% avg (CONDITIONAL PASS; < 80% GA target).
- 168h SOAK only 5min pre-flight PASS; 168h empirical defer to V400-09 follow-up.
- 5/8 WP deferred to v4.0.1.
- V400-08 / V400-10 still 🟡 at v4.0.0 cut.

## ProProvenance（AFP §2.2）

```yaml
provenance:
  generated_by: hybrid (AI + 实跑 gate + git log verify)
  generated_at: 2026-09-29
  input_refs:
    - type: commit
      value: 07178c9d66a45994278742bf90ee86386aae142c
    - type: gate_report
      value: docs/releases/v4.0.0/GA_GATE_REPORT.md
    - type: claim_downgrade
      value: docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md
    - type: soak_baseline
      value: docs/releases/v4.0.0/SOAK_BASELINE_5MIN_2026-09-19.md
  evidence:
    - cargo build --all-features: EXIT=0 (worktree 2026-09-29 18:10Z)
    - cargo fmt --check --all: 76 files drift (deferred — 见 plan §"已知限制")
```