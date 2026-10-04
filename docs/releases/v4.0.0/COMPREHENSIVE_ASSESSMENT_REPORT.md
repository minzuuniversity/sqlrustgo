# SQLRustGo v4.0.0 综合评估报告

> **provenance:** generated_by=codex, generated_at=2026-10-05T00:00:00+08:00, source_repo=openclaw/sqlrustgo, assessed_ref=gitea252/develop/v4.0.0, policy=Anti-Fabrication-Policy-v1.0 + ADR-001 + ADR-014
> **版本**: v4.0.0
> **报告类型**: 横向综合评估；重点是 v4.0.0 相对 v3.12.0 的产品范围扩展，以及 SQL-92 / SQLite / MySQL / PostgreSQL 能力边界
> **阶段背景**: 原 GA CONDITIONAL PASS 判定已于 2026-09-30 作废；本报告不得作为 v4.0.0 GA 证据
> **历史基线**: v3.12.0 scoped GA + v4.0.0 多模型目标
> **证据边界**: 本报告依据 `GA_GATE_REPORT.md` 更正块、`CLAIM_DOWNGRADE_MANIFEST.md`、`LEGACY_ISSUES.md`、`VERSION_PLAN.md`、`V400_09_168H_SOAK_FINAL_REPORT.md` 与本地文档检查；未重新执行 v4.0.0 全量 gate。

## 1. 总体结论

v4.0.0 的产品目标是正确的：把 SQLRustGo 从 v3.12.0 的“GMP 内审检索数据库底座”推进为 SQL + Vector + Graph + Audit/GMP 的多模型数据库。但是，v4.0.0 的 GA 判定已经撤销，原因包括覆盖率 78.28% 未达 RC/GA 阈值、阶段流转未按 STAGE_CONFIG 执行、ALPHA gate 不可执行、人工签字缺失等。因此 v4.0.0 应作为**多模型方向的设计与部分实现基线**阅读，不应作为正式 GA 发布阅读。

横向兼容性结论：

| 对比对象 | v4.0.0 能力边界 | 评估 |
|---|---|---|
| SQL-92 | 继承 v3.12.0 SQL baseline，并继续保留 WP-B/C/D/F/G caveat | 核心 SQL 子集可用，但标准完整性不足 |
| SQLite | 部分 SQLite-style metadata / AUTOINCREMENT / ON CONFLICT / JSON / GROUP_CONCAT 等仍 defer 或 caveat | 不能声明 SQLite 兼容 |
| MySQL | 继承 MySQL wire / SHOW / LOAD DATA 基础能力，目标是 MySQL-compatible relational core | 只能声明 MySQL-like controlled subset，不能声明 MySQL 替代 |
| PostgreSQL | 用作 TPC-H / SQL correctness 对照，不实现 PostgreSQL wire 或生态 | 不具备 PostgreSQL 兼容定位 |
| 多模型数据库 | Vector / Graph / Audit / Cross-model transaction 进入产品目标 | 部分生产代码与计划存在，但完整一致性和 SOAK 不足 |

## 2. 从 v3.12.0 到 v4.0.0 的演进

| 维度 | v3.12.0 | v4.0.0 目标 | v4.0.0 实际边界 |
|---|---|---|---|
| 产品定位 | GMP internal-audit retrieval database workload | SQL + Vector + Graph + GMP multi-model database | 目标成立，但 GA 未成立 |
| SQL | 受控关系数据库子集 | MySQL-compatible relational core | 继承 v3.12，多个 SQL dialect issue defer |
| Vector | 内部 vector/retrieval 能力 | 一等公民 vector storage/index | V400-02 方向成立；大规模恢复和 SQL surface 仍需验收 |
| Graph | SQL-backed graph projection | first-class graph store / traversal | V400-03/04 部分完成；graph ACL/path expressions 等不足 |
| Transaction | SQL 主路径事务能力 | 跨 SQL/vector/graph/audit 原子性 | V400-05 有 tracker/测试，但后续 v4.1 暴露隔离与 GC 风险 |
| Backup/Restore | 关系/GMP 受控能力 | unified backup/restore | V400-06 有 BackupCoordinator；checksum/hash 仍需生产硬化 |
| Audit/ACL | GMP audit chain | ALCOA+ AuditChain + unified ACL | V400-07 有 audit chain；hash/timestamp/ACL 完整性后续升级 |
| SOAK | mixed workload baseline | 168h multi-model SOAK | 5min pre-flight 与 final/deferred 说明，不支持 clean GA |

## 3. SQL-92 支持评估

v4.0.0 没有把 SQL-92 能力提升到完整标准实现。它主要继承 v3.12.0 的 SQL core，并把 v4 工作重心放到 vector / graph / audit / cross-model transaction 上。

| SQL-92 能力 | v4.0.0 状态 | 发布声明边界 |
|---|---|---|
| SELECT / WHERE / ORDER BY / GROUP BY | 继承 v3.12.0 可用子集 | 可声明核心查询子集 |
| JOIN / Subquery | WP-D 多项 defer | 不可声明完整 JOIN/subquery 标准 |
| DDL | WP-C / WP-F 多项 defer | 不可声明完整 schema migration |
| DML | 基础 DML 继承；UPSERT/复杂语义仍有 caveat | 不可声明完整 SQL-92 + MySQL 方言 DML |
| 类型/比较 | WP-G CHAR PAD SPACE defer | 不可声明 MySQL/SQLite 字符比较兼容 |
| 事务 | cross-model tracker 进入生产代码声明，但隔离语义后续仍暴露问题 | 不可声明完整 ACID / isolation |

结论：v4.0.0 的 SQL-92 口径应维持“受控核心子集”，不能因为多模型目标而扩大为“完整 SQL 标准数据库”。

## 4. SQLite 横向比较

v4.0.0 明确把多个 SQLite-style 能力列入必修或 caveat，说明 v3.12 到 v4.0 的主要问题之一正是 SQLite 方言/metadata 兼容未闭环。

| SQLite 能力 | v4.0.0 状态 | 影响 |
|---|---|---|
| `sqlite_master` / `sqlite_sequence` | #4682 defer / backlog | schema introspection 和迁移路径不可宣称兼容 |
| AUTOINCREMENT | #4672 defer | SQLite/MySQL 主键自增语义不完整 |
| ON CONFLICT / UPSERT | #4703 / #4711 caveat | 不能声明 SQLite DML 兼容 |
| JSON_EXTRACT / JSON_EACH | caveat | 不能声明 SQLite JSON 方言 |
| GROUP_CONCAT | caveat | 常见 SQLite 聚合函数缺口 |
| INDEXED BY / expression index | caveat / reevaluate | 查询优化提示与索引方言不完整 |
| 块注释与中文标识 | WP-A 声明 DONE 后被 v4.1 复验推翻 | v4.0 的 WP-A DONE 不应继续当作强证据 |

结论：v4.0.0 不是 SQLite 替代品。它可以面向 GMP/教学场景逐步吸收 SQLite-style 能力，但必须在 release note 中逐项列出 exclusion。

## 5. MySQL 横向比较

v4.0.0 的 SQL database 目标写作“MySQL-compatible relational core”，但其 claim downgrade manifest 已经列出大量 MySQL/MySQL-like 能力边界。

| MySQL 能力面 | v4.0.0 状态 | 边界 |
|---|---|---|
| Wire protocol | 继承 v3.12 MySQL wire 子集 | 可声明受控客户端兼容，不可声明全协议 |
| SHOW / DESCRIBE | 继承 v3.12 controlled subset | information_schema SQL path 仍不足 |
| LOAD DATA | v3.12 起基础能力，v4.0 未作为 clean GA 证据 | SF=1/SF=10、LOCAL、TLS/压缩仍需验证 |
| Stored procedure / trigger | WP-C / v3.12 V312-55 仍有 partial | 不可声明 MySQL stored routine 兼容 |
| ALTER TABLE RENAME COLUMN | #4848 defer | 不可声明 schema migration 兼容 |
| CHAR PAD SPACE | #4846 defer | 主键查找/比较与 MySQL 不一致 |
| Replication / HA | 非 v4.0 完成项 | 不可声明 MySQL 运维替代 |
| Performance schema / admin | 部分能力孤岛 | 不可声明产品级兼容 |

结论：v4.0.0 仍应沿用 v3.10 的“轻量/受控 MySQL 子集”谨慎口径，而且由于 GA 撤销，不能把 v4.0.0 写成 MySQL 替代发布。

## 6. PostgreSQL 横向比较

PostgreSQL 在 v4.0.0 中主要作为性能、MVCC、TPC-H correctness 的行业标尺，而不是协议兼容对象。

| PostgreSQL 能力面 | v4.0.0 状态 | 判断 |
|---|---|---|
| PostgreSQL wire | 不在 v4.0 主线 | 不声明 |
| COPY / bulk load | SQLRustGo 走 MySQL LOAD DATA / BINT / 自有导入路线 | 不声明 COPY 兼容 |
| MVCC / isolation | SQLRustGo 有 MVCC/WAL，但 v4.1 后续暴露隔离缺陷 | 与 PostgreSQL 差距大 |
| Optimizer / statistics | v4.0 有 optimizer/multi-model plan，但成熟度不足 | 不可宣称 PostgreSQL 级优化器 |
| Extensions / catalog | 不具备 PostgreSQL 生态 | 不声明 |
| TPC-H oracle | 可作为正确性参照 | 应继续做 row-count/hash 对比 |

结论：v4.0.0 不能与 PostgreSQL 做兼容性竞争，只能用 PostgreSQL 作为成熟数据库的 correctness/performance baseline。

## 7. 多模型能力评估

v4.0.0 相对 v3.12.0 的真正新增价值是多模型方向：

| Workstream | 目标 | v4.0.0 评估 |
|---|---|---|
| V400-01/02 Vector | vector column/index syntax + WAL-backed vector storage | 方向正确；大规模恢复、索引重建、SQL surface 仍需实测 |
| V400-03/04 Graph | first-class graph storage + query surface | 有图存储和 Cypher subset；MERGE、variable path、graph ACL 不足 |
| V400-05 Cross-model transaction | SQL/vector/graph/audit 统一事务 | tracker/测试存在，但隔离语义与跨模型 crash 证据不足 |
| V400-06 Backup/Restore | unified backup/restore | BackupCoordinator 存在；checksum/hash 策略需加强 |
| V400-07 ACL/Audit | ALCOA+ AuditChain | audit chain 可作为亮点；默认 hash/timestamp 仍需生产级升级 |
| V400-08 Optimizer | multi-model optimizer | ExecutorPool/cost model 部分；不是完整优化器 |
| V400-09 SOAK | 168h multi-model stability | 5min/pre-flight/deferred 证据，不支持 clean GA |

综合判断：v4.0.0 是“多模型数据库路线成型”的版本，但不是“多模型数据库 GA 交付”的版本。

## 8. 主要风险

| 风险 | 严重度 | 说明 |
|---|---|---|
| GA 判定已撤销 | P0 | 所有 v4.0.0 GA 字样必须附更正 |
| 覆盖率低于 RC/GA 阈值 | P0 | 78.28% < RC 80% / GA 85% |
| SQL 方言 caveat 过多 | P0/P1 | SQLite/MySQL 兼容声明必须降级 |
| WP-A DONE 可信度不足 | P0 | v4.1 复验发现 #4708 未修 |
| 多模型事务证据不足 | P0 | v4.1 后续暴露事务隔离/GC P0 |
| 168h SOAK 不可作为 clean PASS | P1 | 文档自身说明 pre-flight/deferred/conditional |
| 大文件与架构债 | P1 | v4.1 继续继承 expr/stored_proc/trigger 拆分问题 |

## 9. 允许与禁止声明

允许声明：

- v4.0.0 是 SQLRustGo 从关系/GMP 底座转向多模型数据库的路线版本。
- v4.0.0 设计目标覆盖 SQL、Vector、Graph、Audit/GMP、Backup/Restore、ACL/Audit。
- v4.0.0 保留了大量 claim downgrade，用于限制 SQLite/MySQL/SQL-92 兼容声明。

禁止声明：

- v4.0.0 已 GA。
- v4.0.0 是 SQLite、MySQL 或 PostgreSQL 替代品。
- v4.0.0 已完整支持 SQL-92。
- v4.0.0 已完成 168h multi-model SOAK clean PASS。
- WP-A、WP-C/D/F/G、#4846/#4847/#4848 已全部修复。

## 10. 对 v4.1.0 的移交建议

v4.0.0 的综合评估结论应直接转化为 v4.1.0 工作：

1. 把 v4.0.0 撤销原因作为 v4.1.0 DRAFT exit checklist。
2. 对 SQL-92 / SQLite / MySQL / PostgreSQL 建立四列表格，不再混用“兼容”一词。
3. 修正 WP-A DONE 假阳性，把 #4708 与未登记 `#[ignore]` 纳入 P0。
4. 将 `CHAR PAD SPACE`、`ALTER TABLE RENAME COLUMN`、事务隔离、AUTO_INCREMENT 作为 MySQL/SQLite 兼容硬阻断。
5. 多模型事务必须用 crash/rollback/backup/restore 组合测试证明，不再只引用设计/单元测试。

## 11. Evidence Log

本报告引用的主要文档证据：

```text
docs/releases/v4.0.0/GA_GATE_REPORT.md
=> 顶部 2026-09-30 更正：原 GA CONDITIONAL PASS 判定作废

docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md
=> 13 项能力边界 + coverage/RSS downgrade

docs/releases/v4.0.0/LEGACY_ISSUES.md
=> v3.12 遗留 SQL issue、caveat 与 v4.0 必修/排除项

docs/releases/v4.0.0/VERSION_PLAN.md
=> v4.0 产品目标：SQL + Vector + Graph + GMP knowledge workloads

docs/releases/v4.0.0/V400_09_168H_SOAK_FINAL_REPORT.md
=> 对 168h SOAK / conditional claim 的边界说明
```
