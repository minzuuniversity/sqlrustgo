# SQLRustGo v4.1.0 — Release Notes

> **Status**: DRAFT (per `docs/releases/v4.1.0/STAGE.yaml`)
> **Branch**: `develop/v4.1.0`
> **Base**: v4.0.0 GA CONDITIONAL PASS (per docs/releases/v4.0.0/GA_GATE_REPORT.md)
> **Scope change vs v4.0.0**: bugfix carry-forward + 168h SOAK FINAL + WP-C..G migration
>
> Per `docs/releases/v4.1.0/STAGE.yaml`, v4.1.0 is in **DRAFT**. v4.0.0 is **not** replaced; v4.0.0 remains GA on its release branch.
>
> ⚠️ **2026-09-30 更正**：本文件原写 `Status: ALPHA (released 2026-09-29,
> tag v4.1.0-alpha1)`，称 v4.1.0 已推进至 ALPHA。该 ALPHA 声明经实跑复核
> 不成立 —— `check_anti_fabrication.sh` 实跑 exit 1，属
> `[AFP-VIOLATION: Type-B 伪门禁]`，已按 AFP §7.1 回退为 DRAFT。
> 详见 `docs/releases/v4.1.0/ALIGNMENT_AUDIT_2026-09-30.md` 与
> `docs/governance/incidents/2026-09-30-V410-ALPHA-UNSUPPORTED-BY-GATE-EVIDENCE.md`。
> 另注：annotated tag `v4.1.0-alpha1` 实际**已存在**且在 develop/v4.1.0 历史中，
> 该 tag 的处置待治理决策（见 STAGE.yaml 顶部说明）。

## 1. What is v4.1.0?

v4.1.0 is the **post-v4.0.0 maintenance continuation** of SQLRustGo. It is
the active development trunk for v4.x.

v4.1.0 carries v4.0.0 forward through:

1. **Real bugfix carry-forward** (already landed on develop/v4.1.0 HEAD):
   - V400-05/06/07 cross-model transaction + AuditChain ALCOA+
   - zombie-fix core (bulk-insert + DLM repair)
   - workers.push wrapper restore in ServerThreadPool::start
   - DML/storage regression test fixes
2. **5-remote sync tooling** (live):
   - scripts/sync/5remotes_sync.sh
   - scripts/sync/5remotes_drift_check.sh
   - scripts/sync/README.md
3. **V400-02 vector WAL coverage** (closed pre-existing test-compile drift):
   - 6 factory functions in crates/storage/src/wal/mod.rs
   - WALOperation extended to 13 variants in crates/wal-verification/src/lib.rs
4. **Refactoring for alpha-gate compliance**:
   - crates/sqlrustgo/src/execution_engine.rs: 2801 → 1017 lines (impl block extracted
     to crates/sqlrustgo/src/execution_engine_methods.rs)

## 2. v4.1.0 vs v4.0.0: stage claim boundary

Per `docs/governance/STAGE_CONFIG.yaml` ALPHA stage:

> Allowed claims (pre-ALPHA): "SQLRustGo v4.1.0 is in active development toward a continuation of v4.0.0."

Allowed claims (pre-BETA): "SQLRustGo v4.1.0-alpha1 has been cut for early testing of v4.0.0 bugfix carry-forward."

Prohibited pre-BETA: "Production multi-model claim (requires GA, and even v4.0.0 GA has 3 caveat items per CLAIM_DOWNGRADE_MANIFEST)"

Prohibited pre-GA: "168h multi-model SOAK result (per V400-09)"

## 3. Known limitations (carried from v4.0.0)

Per `docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md`:

- **SQL surface**: WP-A issues covered; WP-B issues (6: #4721 #4674 #4716 #4676 #4675 #4670) partial; full behavior deferred.
- **DDL surface**: WP-C issues (6: #4682 #4652 #4672 #4669 #4709 #4703) deferred to v4.1.0.
- **Transaction semantics**: WP-E issues (#4847 partial, #4626 not started).
- **Schema**: WP-F #4848 (ALTER TABLE RENAME COLUMN) deferred to v4.1.0.
- **Type/Comparison**: WP-G #4846 (CHAR(n) PAD SPACE) deferred to v4.1.0.

### 3.1 Inherited SQLite-dialect scope exclusions (v4.0.0 §4 — restored 2026-09-30)

> **补回说明**（2026-09-30）：本节原先遗漏 v4.0.0 `LEGACY_ISSUES.md` §4 声明的
> 12 条「继续保持 caveat、不修」issue，其中 11 条在 v4.1.0 全部文档中零出现。
> 按 `GATE_CONDITIONS.md` §2「GA-claim-caveat」判定标准，每条必须在 README /
> RELEASE_NOTES / scope 文档中**显式排除**，否则不得留存为 open。
> 依据：`docs/releases/v4.1.0/ALIGNMENT_AUDIT_2026-09-30.md` F-05。

以下 **11** 条在 v4.1.0 **继续保持 caveat**，不在 v4.1.0 声明范围内：

| Issue | 声明排除的能力 |
|---|---|
| #4719 | sqlite statistics / ANALYZE |
| #4711 | advanced ON CONFLICT forms |
| #4698 | GREATEST/LEAST, math functions |
| #4694 | SET TIMEZONE / isolation syntax |
| #4693 | TEMP/TEMPORARY table syntax |
| #4685 | multi-table UPDATE / DELETE USING |
| #4677 | TRUNCATE and complex LIKE ESCAPE |
| #4667 | ORDER BY NULLS FIRST/LAST |
| #4650 | GROUP_CONCAT and related |
| #4646 | JSON_EXTRACT / JSON_EACH |
| #4625 | INDEXED BY optimizer hint |

**#4670 归属已裁决（2026-09-30，人工决策：算 v4.1.0 的）**

#4670 原在 v4.0.0 `LEGACY_ISSUES.md` 中自相矛盾：§3.4 把它列为「v4.0.0 必修」，
§4 又列为「保持 caveat 不修」。**该冲突已裁决：#4670 归属 v4.1.0**，
按 `CLAIM_DOWNGRADE_MANIFEST.md §2` / §3.1 的 WP-B **partial** 定位处理 ——
即 v4.0.0 声称部分支持，v4.1.0 负责补全到完整行为。

因此 #4670 **已从上方 caveat 排除表中移除**（该表由 12 条变为 11 条），
并计入 `ISSUES_PLAN.md` §4.5「WP-B completion」范围（WP-B 6 条：
#4721 #4674 #4716 #4676 #4675 **#4670**），属 v4.1.0 backlog 20 条之一。

v4.1.0 对 #4670 的声明边界：**当前仍为 partial**，在 WP-B 6 条全部补全前，
不得声称 CEIL/FLOOR/TRUNCATE/HEX/MD5/SHA 已完整支持。

理由：v4.x GA claim 只覆盖 vector + graph + GMP + 关系子集，上述 SQLite
dialect 不在声明范围内。

## 3.2 Build artifact rules (inherited from v4.0.0 §6.2)

> **补回说明**（2026-09-30）：v4.0.0 `LEGACY_ISSUES.md` §6.2 记录的用户
> 2026-09-08 强制声明，在 v4.1.0 文档中零引用。依据
> `ALIGNMENT_AUDIT_2026-09-30.md` F-11。

**强制规则**：禁止提交 `*.log`、`*.tbl`、`*.json` 文件。

- 依据：v3.12.0 GA commit 中 `server.log` 单文件 251MB，超 GitHub 100MB 限制
- 配套 CI gate：`scripts/gate/check_no_log_tbl_json.sh`（**已存在** ✅）
- 配套 `.gitignore` 条目；大 JSON evidence 应转 `.json.gz` 或外部 artifact storage

## 4. v4.1.0 ALPHA scope work (backlog)

Per `docs/releases/v4.1.0/ISSUES_PLAN.md` §4:

| Category | Count | Items |
|---|---|---|
| WP-C DDL / Integrity | 6 | #4682, #4652, #4672, #4669, #4709, #4703 |
| WP-D Join / Subquery | 4 | #4668, #4656, #4649, #4636 |
| WP-F Schema Migration | 1 | #4848 |
| WP-G Type / Comparison | 1 | #4846 |
| WP-B completion | 6 | #4721, #4674, #4716, #4676, #4675, #4670 |
| WP-E completion | 1 | #4626 |
| WP-H #4639 carry-forward | 1 | #4639 |
| **Total** | **20** | 6-10 weeks estimated |

## 5. Stage state

| Stage | Date | Reason |
|------|------|--------|
| DRAFT | 2026-09-23 | Initial entry per STAGE_CONFIG DRAFT_to_ALPHA trigger; PHASE_0 docs scaffolded |
| ~~ALPHA~~ | ~~2026-09-29~~ | **作废** — 声明「3 个 alpha-gate FAIL 全部解决」未经实跑证据支撑（`check_anti_fabrication.sh` 实跑 exit 1）。`[AFP-VIOLATION: Type-B]`。annotated tag `v4.1.0-alpha1` 实际已存在（指向 `c1a73a5320`，由 `liying <openheart@gaoyuanyiyao.com>` 于 2026-09-29 15:11:41 +0800 创建），tag 处置待治理决策。 |
| **DRAFT（回退）** | **2026-09-30** | **按 AFP §7.1 Type-B（P0）回退。3 个继承门禁阻断项现已全部清除并附实跑证据**（`check_anti_ignore_gate` exit 0；`check_arch_invariants` exit 0 PASS 5/5；`check_anti_fabrication` 修复 CHECK 4 白名单后复跑 exit 0）。证据：`evidence/gate-runs-2026-09-30/`。**门禁转绿不等于阶段推进** —— DRAFT → ALPHA 须显式执行 `STAGE_CONFIG` 流程并记录。 |
| BETA | TBD | Blocked by 50% coverage measurement + WP-C..G closure + DRAFT→ALPHA re-entry |
| RC | TBD | Blocked by BETA completion |
| GA | TBD | Blocked by RC completion + 168h SOAK PASS |

## 6. References

- `docs/releases/v4.1.0/STAGE.yaml` — v4.1.0 stage SSOT (current_stage: **DRAFT**)
- `docs/releases/v4.1.0/ALIGNMENT_AUDIT_2026-09-30.md` — v4.0.0→v4.1.0 对齐审计报告
- `docs/governance/incidents/2026-09-30-V410-ALPHA-UNSUPPORTED-BY-GATE-EVIDENCE.md` — AFP Type-B 违规记录
- `docs/releases/v4.1.0/evidence/gate-runs-2026-09-30/` — 门禁实跑输出归档
- `docs/releases/v4.1.0/PHASE_1_SCOPE.md` — DRAFT → ALPHA work plan + resolved items
- `docs/releases/v4.1.0/ISSUES_PLAN.md` §4 — WP-C..G detailed backlog
- `docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md` §3 — v4.0.0 GA caveat items (inherited)
- `docs/releases/v4.0.0/GA_GATE_REPORT.md` — v4.0.0 GA CONDITIONAL PASS verdict
- `docs/releases/v4.0.0/V400_09_168H_SOAK_FINAL_REPORT.md` — SOAK deferral rationale
- `scripts/sync/README.md` — 5-remote sync tooling (v4.1.0 deliverable)
- `docs/governance/STAGE_CONFIG.yaml` — version-agnostic stage framework