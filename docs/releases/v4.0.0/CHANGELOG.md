# SQLRustGo v4.0.0 变更日志

> **状态**: draft (进入 draft phase)
> **日期**: 2026-09-08
> **起点**: `develop/v4.0.0` @ `9febebb255` (= v3.12.0 GA HEAD)
> **维护者**: devops + Release Engineering

---

## v4.0.0-draft (2026-09-08 进入)

### Phase 0 动作

- ✅ `develop/v4.0.0` 分支创建 (从 v3.12.0 GA HEAD `9febebb255`)
- ✅ push 到 4 remote (.252 Gitea, .250 Gitea, gitcode, GitHub*)
  - (\*GitHub 当前 blocked by v3.12.0 GA commit 含 251MB server.log,Phase 0 必须 git filter-branch)
- ✅ `docs/releases/v4.0.0/LEGACY_ISSUES.md` 整理 (21 必修 + 12 caveat + 8 重新评估)
- ✅ `docs/releases/v4.0.0/ROADMAP.md` 4 Phase 路线图 (draft → alpha → beta → rc → ga)
- ✅ `docs/releases/v4.0.0/DEV_PLAN.md` 18 WP + 依赖图 + 估算 76 人周
- ⏳ `.gitignore` 增加 log/tbl/json 排除 (用户 2026-09-08 规则)
- ⏳ `scripts/gate/check_no_log_tbl_json.sh` CI gate
- ⏳ `milestone v4.0.0` Gitea 创建
- ⏳ GMP-Platform PR #207 合并 (self-approval blocker 解决)
- ⏳ branch protection: develop/v4.0.0 draft 配置

### 已规划内容 (沿用 v4.0.0-planned)

- 一等公民 vector column 与 vector index syntax
- WAL-backed vector storage 和 index rebuild
- 一等公民 property graph node/edge storage (基于 sqlrustgo-graph crate)
- graph traversal query surface (Cypher subset + SQL extension)
- SQL、vector、graph 和 GMP audit writes 的 cross-model transaction semantics
- SQL/vector/graph/GMP data 的 unified backup/restore
- unified access control and audit (覆盖所有路径)
- 168h multi-model SOAK

### v3.12.0 遗留必修 (进入 v4.0.0 GA 前必须关闭)

- #4846 CHAR(n) 按字节填充
- #4847 显式事务 3 条路径语义错误
- #4848 ALTER TABLE RENAME COLUMN 不支持
- #4708 Chinese identifier 解析失败
- #4696 UPDATE without WHERE parse failure
- #4710 TIMESTAMPDIFF unit 解析
- #4721 round(real, int) 仍返回 integer
- #4674 CHAR_LENGTH / CHARACTER_LENGTH 错误
- #4672 SQLite AUTOINCREMENT 未生效
- #4682 sqlite_master 缺失
- #4652 CREATE PROCEDURE/FUNCTION 接受但不存储
- #4709 CHECK multi-condition 静默接受非法 row
- #4668 NATURAL JOIN / USING 错误
- #4656 > ALL / = ANY 子query 错误
- #4649 LEFT JOIN USING 退化为笛卡尔积
- #4636 Correlated scalar subquery 失败
- #4626 SELECT FOR UPDATE 后 ROLLBACK 行为
- 以及 17 个 v3.13/defer 重新评估

详见 `LEGACY_ISSUES.md`。

### 强制 commit 规则 (用户 2026-09-08)

- 禁止 `*.log`, `*.tbl`, `*.json` 文件提交 (with allowlist for small config)
- 单文件 < 100MB
- 详细见 `DEV_PLAN.md` §1 + `governance/FILE_GOVERNANCE.md`

### GA 前禁止的声明

- 没有 WAL-backed vector recovery 时,不得宣称 vector database。
- 没有 WAL-backed graph recovery 时,不得宣称 graph database。
- 没有 cross-model transaction tests 时,不得宣称 multi-model production。
- 没有 168h multi-model SOAK 时,不得宣称 GA。

---

## 版本历史

| 版本 | 日期 | 阶段 | 说明 |
|---|---|---|---|
| v4.0.0 | TBD | DRAFT (2026-09-08 → 2026-09-30) | Multi-model SQL + Vector + Graph + GMP database |
| v4.0.0-alpha | TBD (Q4 2026) | planned | First-class vector + storage prototype |
| v4.0.0-beta | TBD (Q1 2027) | planned | Cross-model txn + graph traversal |
| v4.0.0-rc | TBD (Q1 2027) | planned | Unified ops + SOAK + GA prep |
| v4.0.0 (GA) | TBD (Q1 2027) | planned | Production multi-model release |

---

## 附录:英文原文

> 本附录保留本文件改写前的英文原文,便于追溯历史语义;当前正式阅读与执行口径以上方中文正文为准。

# Changelog -- SQLRustGo v4.0.0

> **Status**: DRAFT (entered draft phase 2026-09-08)
> **Date**: 2026-09-08

## v4.0.0-draft

Initial draft entry. v4.0.0 enters draft phase on 2026-09-08 after v3.12.0 GA promotion completed (HEAD `9febebb255`).

### Phase 0 Actions

- Created `develop/v4.0.0` from v3.12.0 GA HEAD `9febebb255`
- Published LEGACY_ISSUES.md (21 must-fix + 12 caveats + 8 re-evaluate)
- Published ROADMAP.md (4 phases: alpha → beta → rc → ga)
- Published DEV_PLAN.md (18 work packages, ~76 person-weeks)
- File governance rule (2026-09-08): no `*.log`, `*.tbl`, `*.json` files in commits; single file < 100MB
- Branch protection on `develop/v4.0.0` to be configured for draft phase

### Planned Capabilities

Same as v4.0.0-planned; see VERSION_PLAN.md.

### v3.12.0 Legacy Issues to Fix

21 must-fix + 8 re-evaluate issues; see LEGACY_ISSUES.md §3 and §5.

### Pre-GA Prohibitions

- Vector DB claim without WAL-backed vector recovery.
- Graph DB claim without WAL-backed graph recovery.
- Multi-model production claim without cross-model transaction tests.
- GA without 168h multi-model SOAK.

## Version History

| Version | Date | Stage | Notes |
|---|---|---|---|
| v4.0.0 | TBD | DRAFT (2026-09-08 → 2026-09-30) | Multi-model SQL + Vector + Graph + GMP database |
| v4.0.0-alpha | TBD (Q4 2026) | planned | First-class vector + storage prototype |
| v4.0.0-beta | TBD (Q1 2027) | planned | Cross-model txn + graph traversal |
| v4.0.0-rc | TBD (Q1 2027) | planned | Unified ops + SOAK + GA prep |
| v4.0.0 (GA) | TBD (Q1 2027) | planned | Production multi-model release |