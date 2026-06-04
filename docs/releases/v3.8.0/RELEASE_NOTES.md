# v3.8.0 RELEASE NOTES (v4 — Route B: Beta → RC → GA, single line)

> **Release**: SQLRustGo v3.8.0 "Architecture Unification & Core SQL"
> **Date**: 2026-06-04
> **Status**: **BETA COMPLETE**, entering **RC** preparation
> **Baseline HEAD**: `052524890` (含 18 milestone PRs)
> **Branch**: `bata/v3.8.0` (just created)
> **Tag**: `v3.8.0-beta` (points to 052524890)
> **Lineage**: v3.7.0 GA → v3.8.0-alpha → v3.8.0-beta → v3.8.0-rc1 → v3.8.0-rc2 → v3.8.0-ga

---

## TL;DR

**路线决策（2026-06-04）**: v3.8.0 走完整生命周期到 GA，**不创建 v3.9.0**。

- v3.8.0-beta 阶段已完成（22 milestone issues, 18 closed by execution, 4 deferred to v3.8.0-rc1）
- v3.8.0-rc 阶段聚焦：性能增强（stable path）+ TPC-H 22/22 完整测试 + 72h 稳定性验证
- v3.8.0-ga 目标：单机数据库 GA（10~100 并发, < 100GB, 单机部署）

**为什么不开 v3.9.0**：
- 当前工作属于"补完与收敛"，不是"新增能力"
- SIMD / Parallel 主路径 / Vector SQL / 新优化器 等明确冻结到 v3.9.0+ 后续版本
- 用户难以理解 "v3.8.0 Beta + v3.9.0 Alpha + v3.9.0 Beta" 连续发布

---

## 1. 版本生命周期 (Route B)

```
v3.7.0 GA  ── 2026-05-30
   │
v3.8.0-alpha  ── 2026-05-31
   │
v3.8.0-beta  ── 2026-06-04  ◀── CURRENT (tag + bata/v3.8.0 分支)
   │
v3.8.0-rc1   ── 计划 2026-06-11 (TPC-H 22/22 + 性能基线)
   │
v3.8.0-rc2   ── 计划 2026-06-18 (72h 稳定性 + 崩溃恢复)
   │
v3.8.0-ga    ── 计划 2026-06-25 (RC gate pass → GA tag)
```

每个阶段都有：gate contract → 报告 → tag → 公告。

---

## 2. v3.8.0 Beta 阶段交付清单

### 2.1 22 Milestone Issues (18 closed, 4 deferred to RC)

| # | Issue | 状态 | PR |
|---|-------|------|-----|
| #2966 | INT-1 DML WAL | ✅ | #2995, #3019 |
| #2967 | EXEC-01 GROUP BY | ✅ | #2998 |
| #2968 | EXEC-02 JOIN | ✅ | #3000 |
| #2969 | EXEC-03 Aggregates | ⏭→RC | (opencode: 0% mod-tree 修复) |
| #2970 | EXEC-04 HAVING | ✅ | #2998 |
| #2971 | EXEC-05 NULL | ✅ | #2997 |
| #2972 | EXEC-06 DISTINCT | ✅ | #3002 |
| #2973 | INT-4 VtuGuard | ✅ | #2999 |
| #2974 | ARCH-2 merge.rs | ✅ | #3001 |
| #2975 | SEM-1 语义 | ✅ | #3008 |
| #2976 | SEM-2 错误码 | ✅ | #3011 |
| #2977 | TPCH-01 (10→22) | ⏭→RC | (opencode 进行中) |
| #2978 | CLI-01 P0 commands | ✅ | #3003 |
| #2979 | CLI-02 P1 commands | ✅ | #3007 |
| #2980 | SERVER-01 验收 | ✅ | #3012 |
| #2986 | INT-2 I-12 Parallel | ⏭→v3.9.0 | #3014 (Phase 1 SPEC) |
| #2987 | CTE-01 (10 fails) | ⏭→v3.9.0 | (opencode) |
| #2988 | MySQL-01 高级函数 | ✅ | #3022 (PARTIAL: 6/26) |
| #2989 | PERF-01 sysbench | ⏭→v3.9.0 | #3015 (Phase 1 SPEC) |
| #2990 | VEC-01 Vector SQL | ⏭→v3.9.0 | #3016 (Phase 1 SPEC) |
| #2991 | EXEC-07 F-10/F-14 | ✅ | #3017 (F-10 only, F-14 已存在) |
| #2992 | DOCS-01 CHANGELOG | ✅ | #3005 |

**新阶段分类**:
- 18 closed → Beta 完成
- 2 deferred to RC (#2969 Aggregates, #2977 TPCH) — 必须 v3.8.0-rc1 解决
- 4 deferred to v3.9.0+ (#2986/#2987/#2989/#2990/#2988 ROLLUP/CUBE/INSERT) — 明确不属于 v3.8.0

### 2.2 测试现状 (Beta 阶段)

```
sqlrustgo-executor --lib       351 passed (was 334, +17 MySQL-01 functions)
sqlrustgo-types --lib           84 passed (was 81, +3 SEM-2 error code)
tests/distinct_test              5 passed (CLI-02 follow-up)
tests/describe_table_test        3 passed (CLI-02)
tests/multi_join_test            4 passed (EXEC-07)
tests/mysql_functions_test       4 passed (MySQL-01)
tests/tpch_value_correctness     3 passed, 1 ignored (Q3 parser)
tests/tpch_gate                  1 passed
                          ──────────────
                            455 unit + integration tests
```

### 2.3 治理资产 (Beta 完成)

- ✅ `docs/governance/EXECUTION_SEMANTICS.md` — 9-clause contract
- ✅ `scripts/gate/check_execution_semantics.sh` — 4-check verifier, PASS
- ✅ `scripts/gate/update_changelog.sh` — CHANGELOG auto-update (DOCS-01)
- ✅ `docs/governance/EXECUTION_SEMANTICS.md` — SEM-1 contract
- ✅ `SqlError::mysql_error_code() / sqlstate()` — SEM-2 standardization
- ✅ 4 跟踪 issue 已更新 + 1 评审 issue (#2702) 已 closed

### 2.4 文档资产 (Beta 完成)

- `docs/releases/v3.8.0/verify/SERVER_01_ALPHA_ACCEPTANCE.md` — Server 验收报告
- `docs/releases/v3.8.0/specs/gate/INT2_PARALLEL_EXECUTOR_SPEC.md` — Phase 1 SPEC (deferred to v3.9.0)
- `docs/releases/v3.8.0/specs/gate/PERF01_SYSBENCH_SPEC.md` — Phase 1 SPEC (deferred to v3.9.0)
- `docs/releases/v3.8.0/specs/gate/VEC01_VECTOR_STORE_SPEC.md` — Phase 1 SPEC (deferred to v3.9.0)
- `docs/releases/v3.8.0/V380_TO_V390_HANDOVER.md` — 交接文档（路径冻结）

---

## 3. v3.8.0-rc1 阶段计划 (2026-06-04 → 2026-06-11)

### 3.1 必须完成 (RC1 gate block)

1. **TPC-H 22/22 全通过** (#2977) — RC gate 硬性要求
2. **EXEC-03 Aggregates** (#2969) — mod-tree dead code 修复或移除
3. **MySQL-01 ROLLUP/CUBE/INSERT/REPLACE** (#2988 续) — 26 fails 中其余 20 个
4. **RECOVERY_TEST_DESIGN.md 验证** — 崩溃恢复场景覆盖
5. **72h 稳定性测试** — 内存泄漏 / 死锁 / WAL 一致性

### 3.2 性能基线

- 基准：TPC-H SF=0.01 / SF=0.1 / SF=1 三档
- sysbench OLTP_READ_WRITE 1/4/16 threads (Phase 1)
- 内存: 启动 100MB → 8h 持续 500MB → 72h 持续 800MB 上限

### 3.3 RC1 gate contract

- `bash scripts/gate/check_execution_semantics.sh` PASS
- `cargo test --workspace` 0 failures
- TPC-H 22/22 (no SKIP)
- 72h 稳定性报告
- DEPLOYMENT_GUIDE.md 验证

---

## 4. v3.8.0-rc2 阶段计划 (2026-06-11 → 2026-06-18)

- 复测所有 v3.8.0-rc1 deliverables
- Crash recovery 极端场景 (kill -9 / OOM / 断电)
- Wire protocol 完整 e2e (MySQL client 真实连接)
- README + DEPLOYMENT_GUIDE final

---

## 5. v3.8.0-ga 阶段计划 (2026-06-18 → 2026-06-25)

- RC2 gate pass → 打 `v3.8.0-ga` tag
- main 分支同步 v3.8.0-ga（需要 PM 授权 — 治理红线）
- CHANGELOG.md final
- Release announcement (RELEASE_NOTES v5 = GA)

---

## 6. v3.9.0 路线图 (明确冻结)

| 模块 | v3.9.0 计划 |
|------|------------|
| SIMD Executor | 新增 |
| Parallel Executor 主路径 | 集成 (Phase 2-4) |
| Vector SQL 表面 | 集成 (Phase 2-5) |
| 新优化器 | 重构 |
| MySQL 高级函数扩展 | ROLLUP / CUBE / INSERT / REPLACE / RANK / DENSE_RANK 等 |
| CTE Recursive | 解析器 |
| sysbench MySQL 5.7 比较 | Phase 2-3 |

**不在 v3.8.0 范围**：以上所有。明确冻结到 v3.9.0 (后续版本)。

---

## 7. 治理红线

1. **禁止手动关闭没有 PR 合并的 Issue** (AGENTS.md)
2. **修改 docs/ 必须 5 步流程** (DOC_CHECK_CORRECTION_RULES.md)
3. **CRITICAL: main 分支不直接 push** — 仅 GA tag 后可同步
4. **Gitea token 走 `~/.git_token_gitea`**

---

## 8. References

- Issue: https://192.168.0.252:3000/openclaw/sqlrustgo/issues (v3.8.0 milestone)
- Branch: `bata/v3.8.0` (origin)
- Tag: `v3.8.0-beta` (052524890)
- Semantics: `docs/governance/EXECUTION_SEMANTICS.md`
- Alpha report: `docs/releases/v3.8.0/verify/SERVER_01_ALPHA_ACCEPTANCE.md`
- Legacy fix verification (PR #2701, reviewed 2026-06-04)
- v3.7.0 GA reference: `docs/releases/v3.7.0/CHANGELOG.md`
