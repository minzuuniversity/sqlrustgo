# v3.11.0 Post-GA 计划

## 1. 计划定位

本文件记录 v3.11.0 GA 后的后续工作。Post-GA 任务不是 GA 已通过证据，而是 v3.11.x patch、v3.12.0 或 v4.0.0 的输入。

## 2. 优先事项

| 优先级 | 工作 | 说明 |
|---|---|---|
| P0 | TPC-H correctness close-out | 对 zero-row query 做跨引擎 row-count/SHA256 对比 |
| P0 | Coverage methodology | 统一 G3 覆盖率命令，消除 `--lib` / `--tests` 混用 |
| P0 | SQLLogicTest gate | 恢复 v3.10 规划的 SQLite SQLLogicTest 自动测试，并集成到 v3.12 gate |
| P0 | MySQL wire protocol | 补 COM_STMT、error packet、reset、TLS/compression E2E |
| P1 | LOAD DATA / recovery / upgrade | 形成 row/hash、restore、rollback 证据 |
| P1 | GMP/RAG/Vector/Graph | 为 `~/gmp-platform` 建立受控生产契约 |

## 3. 声明边界

Post-GA 计划中的条目不得写成已完成，除非对应 PR、commit、命令输出和 evidence hash 已存在。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义。若英文附录与中文正文或 `COMPREHENSIVE_ASSESSMENT_REPORT.md` 冲突，当前正式判断以中文正文和综合评估报告为准。

# v3.11.0 Pre-GA Plan (整改后)

> **Version**: v3.11.0
> **Status**: PRE-GA (2026-07-20 整改后) — **GA 未发布**；G3/G4 FAIL；待 P0 整改完成
> **Owner**: @openclaw

Pre-GA tasks and remaining P0 items (按 `TPCH_SF1_VERIFICATION_REPORT.md` + `AUDIT_V311_REALITY_CHECK.md`)。

---

## 1. v3.11.0 Post-GA Immediate Tasks

| Task | Description | Status |
|------|-------------|--------|
| Tag v3.11.0 | Create git tag and release | Pending |
| Binary builds | Build release binaries for all platforms | Pending |
| Publish crates | `cargo publish` for all workspace crates | Pending |
| 168h SOAK | 168-hour stress test post-release | ✅ **DONE (本机实测 343h37m,2.04x 168h,0 errors)** — 详见 `SOAK_168H_REPORT.md` |

---

## 2. v3.12.0 Planning

| Feature | Description | Priority |
|---------|-------------|----------|
| Distributed query execution | Multi-node query planning | P1 |
| Cypher compatibility | Graph traversal syntax | P2 |
| SIMD vectorization | Batch processing optimization | P2 |

---

## 3. Coverage Improvement Roadmap

| Crate | Current | v3.12.0 Target | Notes |
|-------|---------|----------------|-------|
| sqlrustgo-mysql-client | 43.79% | 60% | Integration tests |
| sqlrustgo-tools | 63.84% | 75% | CLI integration tests |
| sqlrustgo-mysql-server | 51.53% | 70% | Protocol tests |
| sqlrustgo-parser | 71.22% | 80% | Parser edge cases |
| sqlrustgo-executor | 76.45% | 82% | Executor edge cases |

---

## 4. Technical Debt

| Item | Description | Track Issue |
|------|-------------|-------------|
| Extension crate cleanup | Finalize V311-19 decisions | #3612 |
| API drift tests | Fix 12 broken integration tests | #3421 |
| BufferPool refactor | Remove legacy BufferPoolWithClock | #3136 |
