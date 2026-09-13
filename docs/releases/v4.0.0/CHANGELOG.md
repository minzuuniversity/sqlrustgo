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
- ✅ `docs/releases/v4.0.0/DEV_PLAN.md` 19 WP + 依赖图 + 估算 80 人周
- ✅ `docs/releases/v4.0.0/GMP_PLATFORM_REQUIREMENTS.md` 新增 GMP-Platform v1.5/v1.6 consumer contract
- ✅ `docs/releases/v4.0.0/TEST_PLAN.md` 扩展到 V400-G11/G12,覆盖 legacy regression 与 GMP consumer gate
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
- V400-10 GMP-Platform consumer regression: compile,408,REST/WebUI,audit,CJK,upload smoke

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
- 没有 V400-10 consumer evidence 时,不得宣称满足 GMP-Platform v1.5/v1.6 全量需求。
- 408/WebUI/audit 失败未分离 SQLRustGo/GMP/corpus/LLM 责任时,不得用作 GA pass 证据。

### Phase 0 之后的修复

#### V400-#95 — STMT PREPARE 在 `CLIENT_SESSION_TRACK` 下 2027 "Malformed packet" 回归

**症状 (sysbench)**: 每次 prepare step 都报 `SQL error, errno = 2013, state = 'HY000': Lost connection to MySQL server during query` 与 `2027 Malformed packet`。已与 MySQL 8.0.46 wire bytes 对比,本地用 `sysbench /usr/share/sysbench/oltp_read_write.lua --mysql-db=test prepare` 复现。

**根因**: libmysqlclient (sysbench 通过 `mysql_stmt_prepare()` 使用) 解析 COM_STMT_PREPARE 响应的尾部 OK packet 时,**只要 `CLIENT_SESSION_TRACK` 已协商,就期望 `warnings` 之后出现 lenenc `info` 字节——无论 `SERVER_STATUS_SESSION_STATE_CHANGED` (0x4000) 是否设置**。两个 OK packet 都缺这个字节:

1. **首个 OK packet** (携带 `stmt_id` / `column_count` / `param_count`): 之前 12 字节,libmysqlclient 把下个 packet 的 header 字节当作 `info` 长度 → 失同步 → 2027。
2. **Result-set terminator** (deprecate-EOF OK `0xFE …`): `0x4000` 未设时仅 7 字节,libmysqlclient 同样把下个 packet 首字节当作 `info` 长度 → 失同步 → 2027。

**修复** (`crates/mysql-server/src/lib.rs`):

1. 将内联 STMT_PREPARE OK builder 提取为 `make_stmt_prepare_initial_ok_packet(stmt_id, column_count, param_count, client_cap)`。在 `client_cap & SESSION_TRACK != 0` 时写 lenenc `info=0`。SESSION_TRACK 下变为 13 字节 (12 base + 1 info),无 SESSION_TRACK 时仍为 12 字节 (legacy client 不受影响)。
2. 重构 `make_deprecate_eof_ok_packet`: 把尾部字节条件拆分——`lenenc(info=0)` 在 SESSION_TRACK 协商时无条件写 (与 `make_ok_packet` 在 lib.rs:2200-2210 一致),`lenenc(session_state_changes)` 仅在 `0x4000` 设置时写。Terminator wire shape:
   - 7 字节: 无 SESSION_TRACK (legacy 形状不变)。
   - 8 字节: SESSION_TRACK + 无 0x4000 (回归场景——已修复)。
   - 9 字节: SESSION_TRACK + 0x4000 (行为不变,offset 7 多 1 字节 `info`)。

**测试** (`stmt_prepare_terminator_tests` 模块 + 4 个更新的 `integration_tests`):

- `deprecate_eof_terminator_includes_info_field_when_session_track_negotiated` (RED → PASS): `status=0x0002` 时 8 字节 terminator。
- `deprecate_eof_terminator_omits_info_field_when_session_track_not_negotiated`: 7 字节 terminator 保留 (legacy 形状)。
- `deprecate_eof_terminator_includes_info_and_session_state_when_session_changed`: `status=0x4002` 时 9 字节 terminator。
- `deprecate_eof_terminator_matches_ok_packet_info_field_under_session_track`: 交叉校验 `make_ok_packet` 与 `make_deprecate_eof_ok_packet` 在 SESSION_TRACK 下尾部 `info` 字节一致。
- `stmt_prepare_initial_ok_includes_info_field_when_session_track_negotiated` (RED → PASS): SESSION_TRACK 下 13 字节 initial OK。
- `stmt_prepare_initial_ok_omits_info_field_when_session_track_not_negotiated`: 12 字节 initial OK 保留。

6 个测试修复后全部 PASS。`mysql-server` crate 全量: 250 passed / 0 failed。

#### V400-#94 — execute_update / execute_delete O(N) Vec clones 二次泄漏 (`239c00533f`)

LEAK-DIAG instrumentation 移除后泄漏从 ~820 MB/h 降到 ~480 MB/h,但 jemalloc heap dump 仍显示 `execute_update` 占 32.4% inuse、`run_before_update_triggers` 占 20.8%。三个独立泄漏源已修复: DELETE WHERE 路径 `scan_with_filter`、`execute_update` no-WHERE 路径行级 clone、`run_before_update_triggers` 旁路 `Cow<[_]>`。5min sysbench oltp_read_write 8-thread 10000-row SOAK 后,RSS 平稳在 19.2 MB,不再以 ~17 MB/min 线性增长。

#### V400-pool-sat — 背压超时返回 `ER_CON_COUNT_ERROR (1040)` (`dad6018299`)

连接池饱和且 `get_with_timeout` 超过配置的背压截止时间时,服务端改为向客户端返回 MySQL `ER_CON_COUNT_ERROR (1040)`,而非直接断开连接。与 MySQL 8.0 在 `max_connections` 超限时的线行为匹配,客户端可据此实现 backoff/retry。

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

### Fixes since Phase 0

#### V400-#95 — STMT PREPARE "Malformed packet" (2027) regression under `CLIENT_SESSION_TRACK`

**Symptom (sysbench)**: every prepared-statement prepare step emitted `SQL error, errno = 2013, state = 'HY000': Lost connection to MySQL server during query` and `2027 Malformed packet`. Confirmed against MySQL 8.0.46 wire bytes and reproduced locally with `sysbench /usr/share/sysbench/oltp_read_write.lua --mysql-db=test prepare`.

**Root cause**: libmysqlclient (used by sysbench via `mysql_stmt_prepare()`) parses the trailing OK packet of the COM_STMT_PREPARE response and EXPECTS a lenenc `info` byte after `warnings` whenever `CLIENT_SESSION_TRACK` is negotiated — REGARDLESS of whether `SERVER_STATUS_SESSION_STATE_CHANGED` (0x4000) is set in `status_flags`.

1. **First OK packet** (carries `stmt_id` / `column_count` / `param_count`): previously 12 bytes; libmysqlclient read the next packet's header byte as the `info` length → desync → 2027.
2. **Result-set terminator** (the deprecate-EOF OK packet `0xFE …`): previously 7 bytes when `0x4000` was unset; libmysqlclient read the next packet's first byte as `info` length → desync → 2027.

**Fix** (`crates/mysql-server/src/lib.rs`):

1. Extracted the inline STMT_PREPARE OK builder into `make_stmt_prepare_initial_ok_packet(stmt_id, column_count, param_count, client_cap)`. The new helper writes the unconditional `lenenc(info=0)` byte when `client_cap & SESSION_TRACK != 0`. Payload becomes 13 bytes (12 base + 1 info) under SESSION_TRACK, stays 12 bytes without it (legacy clients unaffected).
2. Refactored `make_deprecate_eof_ok_packet`: split the trailing-byte condition so the `lenenc(info=0)` byte is written whenever SESSION_TRACK is negotiated (matching `make_ok_packet`'s semantics at lib.rs:2200-2210), while `lenenc(session_state_changes)` is still written ONLY when `0x4000` is set. Terminator wire shape:
   - 7 bytes: no SESSION_TRACK (legacy clients unchanged).
   - 8 bytes: SESSION_TRACK + no 0x4000 (the regression case — fixed).
   - 9 bytes: SESSION_TRACK + 0x4000 (unchanged behavior, now with the additional `info` byte at offset 7).

**Tests** (`stmt_prepare_terminator_tests` module + 4 updated `integration_tests`):

- `deprecate_eof_terminator_includes_info_field_when_session_track_negotiated` (RED → PASS): 8-byte terminator with `status=0x0002`.
- `deprecate_eof_terminator_omits_info_field_when_session_track_not_negotiated`: 7-byte terminator preserved (legacy wire shape).
- `deprecate_eof_terminator_includes_info_and_session_state_when_session_changed`: 9-byte terminator with `status=0x4002`.
- `deprecate_eof_terminator_matches_ok_packet_info_field_under_session_track`: cross-check that `make_ok_packet` and `make_deprecate_eof_ok_packet` agree on the trailing `info` byte under SESSION_TRACK.
- `stmt_prepare_initial_ok_includes_info_field_when_session_track_negotiated` (RED → PASS): 13-byte initial OK with `SESSION_TRACK`.
- `stmt_prepare_initial_ok_omits_info_field_when_session_track_not_negotiated`: 12-byte initial OK preserved.

All 6 tests PASS post-fix. Full `mysql-server` crate test suite: 250 passed / 0 failed.

#### V400-#94 — Secondary leak: execute_update / execute_delete O(N) Vec clones (`239c00533f`)

LEAK-DIAG instrumentation removal cut the leak from ~820 MB/h to ~480 MB/h, but jemalloc heap dumps still showed 32.4% inuse in `execute_update` and 20.8% inuse in `run_before_update_triggers`. Three independent leak sources fixed: `scan_with_filter` for the DELETE WHERE path, row-level cloning in `execute_update`'s no-WHERE path, and a `Cow<[_]>` bypass around `run_before_update_triggers`. After 5-min sysbench oltp_read_write 8-thread 10000-row SOAK, RSS now stays flat near 19.2 MB instead of climbing ~17 MB/min linearly.

#### V400-pool-sat — `ER_CON_COUNT_ERROR (1040)` on backpressure timeout (`dad6018299`)

When the connection pool is saturated and a `get_with_timeout` exceeds the configured backpressure deadline, the server now returns MySQL `ER_CON_COUNT_ERROR (1040)` to the client instead of dropping the connection. Matches the MySQL 8.0 wire behavior observed when `max_connections` is exceeded, so clients can implement sensible backoff/retry.

## Version History

| Version | Date | Stage | Notes |
|---|---|---|---|
| v4.0.0 | TBD | DRAFT (2026-09-08 → 2026-09-30) | Multi-model SQL + Vector + Graph + GMP database |
| v4.0.0-alpha | TBD (Q4 2026) | planned | First-class vector + storage prototype |
| v4.0.0-beta | TBD (Q1 2027) | planned | Cross-model txn + graph traversal |
| v4.0.0-rc | TBD (Q1 2027) | planned | Unified ops + SOAK + GA prep |
| v4.0.0 (GA) | TBD (Q1 2027) | planned | Production multi-model release |
