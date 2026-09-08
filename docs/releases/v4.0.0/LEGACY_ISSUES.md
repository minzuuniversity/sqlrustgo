# SQLRustGo v4.0.0 — v3.12.0 Legacy Issues Handover

> **版本**: v4.0.0 (draft phase)
> **编制日期**: 2026-09-08
> **来源**: v3.12.0 GA gate evidence + open issues + RC/GA triage document
> **目标**: 把 v3.12.0 阶段遗留的 SQL 缺陷作为 v4.0.0 必修项或可声明的边界条件

---

## 1. 来源文档

| 文档 | 用途 |
|---|---|
| `docs/releases/v3.12.0/GA_GATE_REPORT.md` | 8 项 GA promotion 项执行证据 |
| `docs/releases/v3.12.0/RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md` | GA-blocker / GA-claim-caveat / v3.13/defer 三分类 |
| `docs/releases/v3.12.0/CLAIM_DOWNGRADE_MANIFEST.md` | scope 缩减声明 |
| `docs/releases/v3.12.0/RC_GATE_REPORT.md` | RC 阶段 verdict |
| `docs/releases/v3.12.0/b2-disabled-test-binary-registry.md` | 关闭测试清单 |
| `docs/releases/v3.12.0/sqllogictest-oracle-gate-report.md` | SQLLogicTest 结果 |

---

## 2. v3.12.0 GA 后状态

- `develop/v3.12.0` HEAD: `9febebb255` (`fix(v312 / GA post-cut verification)`)
- `main` 与 `release/v3.12.0`: 同 HEAD `9febebb255`
- `v3.12.0` milestone: closed 83 issues, 0 open
- open issues 不属于 v3.12.0 milestone:**3 个**(#4846, #4847, #4848)

### v3.12.0 GA 实际保留范围

> SQLRustGo v3.12.0 是面向 GMP internal-audit retrieval workload 的 production-ready 关系数据库 + 内部 vector + 内部 graph projection。

未声明的能力被显式降级 (downgrade) 而非修复:

- MySQL user variables, JSON_EXTRACT, GROUP_CONCAT 等 advanced SQL dialect
- 完整 recursive CTE, window function frame, generated columns
- 触发器运行时 (FOR EACH STATEMENT)
- multi-table UPDATE/DELETE USING
- RIGHT/FULL OUTER JOIN (只声明 INNER/LEFT)

---

## 3. v4.0.0 必须解决的 v3.12.0 遗留 (必修)

下列 issue 在 v3.12.0 GA 阶段被暂时声明 caveat,但 **进入 v4.0.0 GA 时必须 fix**,否则 v4.0.0 GA 声明边界不可用。

### 3.1 DDL / 元数据

| Issue | 标题 | 影响 v4.0.0 |
|---|---|---|
| #4682 | `sqlite_master` / `sqlite_sequence` / `sqlite_temp_master` 缺失 | v4.0.0 必须支持 sqlite-style metadata,否则迁移路径不可用 |
| #4652 | CREATE PROCEDURE/FUNCTION 接受但不存储 | DDL fake-success;v4.0.0 first-class graph/vector 依赖 procedure/function 框架 |
| #4672 | SQLite AUTOINCREMENT 仍未生效 | identity column 是 v4.0.0 metadata / audit event 主键基础 |
| #4669 | 复杂 DROP INDEX / function index / partial index | v4.0.0 vector index 与 expression index 需要 DDL 扩展 |

### 3.2 数据完整性

| Issue | 标题 | 影响 v4.0.0 |
|---|---|---|
| #4709 | CHECK multi-condition 静默接受非法 row | data integrity;v4.0.0 multi-model 写同一 transaction 不可绕过 CHECK |
| #4703 | UPSERT / trigger-column syntax failures | v4.0.0 audit-event 写入需要 idempotent insert |

### 3.3 Parser / Identifier

| Issue | 标题 | 影响 v4.0.0 |
|---|---|---|
| #4708 | Chinese identifier / comment / quoted identifier 解析失败 | GMP 语料以中文为主,v4.0.0 GMP layer 必须支持 |
| #4696 | UPDATE without WHERE parse failure | basic DML 阻塞,v4.0.0 multi-model path 不可用 |
| #4720 | MySQL user variables | GMP audit-event 注入参数化需要 |
| #4710 | TIMESTAMPDIFF unit 解析 | v4.0.0 audit-event 时序差 |

### 3.4 类型 / 函数

| Issue | 标题 | 影响 v4.0.0 |
|---|---|---|
| #4721 | `round(real, int)` 仍返回 integer | GMP 数值字段精度不可靠 |
| #4674 | CHAR_LENGTH / CHARACTER_LENGTH 返回错误 | v4.0.0 metadata column 长度计算 |
| #4716 | DATE_TRUNC 部分实现 | v4.0.0 audit-event 时间窗口聚合 |
| #4676 | MOD/POWER/LOG/EXP/SQRT 缺失 | v4.0.0 cost/embedding 数值运算 |
| #4675 | POSITION/LOCATE 缺失 | v4.0.0 chunk / document 文本定位 |

### 3.5 Join / Subquery

| Issue | 标题 | 影响 v4.0.0 |
|---|---|---|
| #4668 | NATURAL JOIN / multi-column USING 错误 | v4.0.0 graph traversal SQL projection |
| #4656 | `> ALL` / `= ANY` 子query 错误 | GMP 检索子句 |
| #4649 | LEFT JOIN USING 退化为笛卡尔积 | v4.0.0 关联查询正确性 |
| #4636 | Correlated scalar subquery 失败 | v4.0.0 GMP 检索核心模式 |

### 3.6 事务 (P1 — v4.0.0 必须修复)

| Issue | 标题 | 影响 v4.0.0 |
|---|---|---|
| #4847 | 显式事务 3 条路径语义错误 (batch 注释行 / ROLLBACK 误回滚 / wire abort) | **v4.0.0 多模型事务一致性的硬阻断** — 不能跨模型 atomic |
| #4626 | SELECT FOR UPDATE 后 ROLLBACK 行为可能破坏事务正确性 | v4.0.0 audit chain 一致性 |

### 3.7 类型 / 比较

| Issue | 标题 | 影响 v4.0.0 |
|---|---|---|
| #4846 | CHAR(n) 按字节填充导致主键点查 0 行 | **v4.0.0 GMP 主键与 SQLite/MySQL 不一致** |

### 3.8 Schema 修改

| Issue | 标题 | 影响 v4.0.0 |
|---|---|---|
| #4848 | ALTER TABLE RENAME COLUMN 不支持 | **v4.0.0 schema migration 阻断** — multi-version migration 必须 |

---

## 4. v3.12.0 已显式降级 + v4.0.0 仍保持 caveat (可继续 exclude)

下列在 v3.12.0 中被 declare caveat,在 v4.0.0 中**继续保持** caveat (即不修):

- #4719 sqlite statistics / ANALYZE
- #4711 advanced ON CONFLICT forms
- #4698 GREATEST/LEAST, math functions
- #4694 SET TIMEZONE / isolation syntax
- #4693 TEMP/TEMPORARY table syntax
- #4685 multi-table UPDATE / DELETE USING
- #4677 TRUNCATE and complex LIKE ESCAPE
- #4670 CEIL/FLOOR/TRUNCATE/HEX/MD5/SHA partial
- #4667 ORDER BY NULLS FIRST/LAST
- #4650 GROUP_CONCAT and related
- #4646 JSON_EXTRACT / JSON_EACH
- #4625 INDEXED BY optimizer hint

理由:v4.0.0 GA claim 只覆盖 vector + graph + GMP + 关系子集,这些 dialect 不在声明范围内。

---

## 5. v4.0.0 需要重新评估 (从 v3.13/defer 升级)

v3.12.0 GA 时被推迟到 v3.13/v4.0 的 issue,v4.0.0 必须重新评估:

| Issue | 推迟原因 | v4.0.0 处理 |
|---|---|---|
| #4717 | INSERT with recursive CTE | v4.0.0 graph node/edge 批量 insert 需要 |
| #4707 | LEAD/LAG/FIRST_VALUE/LAST_VALUE | v4.0.0 audit-event window 聚合 |
| #4701 | Expression indexes | v4.0.0 vector index / metadata 索引 |
| #4692 | Writable CTE | v4.0.0 multi-model write 路径 |
| #4699 | Recursive CTE execution | v4.0.0 graph traversal native SQL surface |
| #4688 | CREATE SEQUENCE | v4.0.0 audit-event ID generation |
| #4671 | CREATE FUNCTION multi-statement body / RETURNS TABLE | v4.0.0 GMP stored function runtime |
| #4639 | RIGHT/FULL OUTER JOIN | v4.0.0 关系图 join 完整性 |

每个都必须重新在 v4.0.0 WP (Work Package) 中分配 owner + exit evidence。

---

## 6. 测试与构建系统遗留

### 6.1 测试目录与 SOAK

- `docs/releases/v3.12.0/b2-disabled-test-binary-registry.md` 列出 116+ 关闭测试 (cap 125)
- `docs/releases/v3.12.0/sqllogictest-baseline/` 保留 SQLite/MySQL oracle comparison 基线
- 168h SOAK evidence 在 `docs/releases/v3.12.0/evidence/MIXED_SOAK_REPORT.md`

### 6.2 Build artifact 大文件问题

v3.12.0 GA commit (`.250 backup`) 中 `server.log` 单文件 251MB,GitHub 100MB 限制无法直接推送。

**v4.0.0 强制规则** (用户 2026-09-08 声明):

> 禁止 `*.log`、`*.tbl`、`*.json` 文件提交。

应对措施:
- `docs/releases/v3.12.0/evidence/**/server.log` 应转入 git LFS 或外部 artifact storage
- `.gitignore` 加入 `*.log`, `*.tbl`, `*.json`
- 大 JSON evidence 转 `.json.gz` 或 external artifact
- CI gate 增加 file size + extension check (`scripts/gate/check_no_log_tbl_json.sh`)

### 6.3 PR/branch 治理

- `docs/governance/BRANCH_GOVERNANCE.md` 完整;v4.0.0 沿用 develop/v4.0.0 + release/v4.0.0 + ga/v4.0.0 模式
- 保留 develop/*, release/*, ga/*, alpha/*, beta/*, rc/* 前缀;不清理
- 强制清理 merged work branches

---

## 7. 总结

| 类别 | 数量 | v4.0.0 处理 |
|---|---|---|
| v4.0.0 必修 issue (Section 3) | 21 | 进入对应 WP,必有 merged PR + regression test |
| v4.0.0 维持 caveat (Section 4) | 12 | 显式 scope exclusion |
| v4.0.0 重新评估 (Section 5) | 8 | 重新 triage,部分进入 WP |
| Build / 测试 / 治理 | - | 沿用 v3.12.0 框架 + 增加 log/tbl/json gate |

**进入 v4.0.0 RC 的前置条件**:

- Section 3 全部 21 issue closed via merged PR
- Section 5 至少 6 / 8 升级到 WP (按 v4.0.0 product scope 决定)
- `*.log`、`*.tbl`、`*.json` gate scripts 已就绪
- 168h multi-model SOAK 计划已就位