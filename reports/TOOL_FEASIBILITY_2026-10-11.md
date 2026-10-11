# 12-Tool Feasibility Check（ChatGPT 工具选型清单）

> **作者**：deepseek-flash（DeepSeek-Harness）
> **日期**：2026-10-11
> **依据**：用户转发的 ChatGPT 工具选型建议（12 款 + SQLancer/sqllogictest/flamegraph = 15 项）
> **HEAD evaluated**：`develop/v4.1.0` tip `01df92a5d1`（worktree `feature/audit-storage-mutation`）
> **scope**：仅验证 **可装性**（`--version` 能跑），**不**实际跑分析

## 1. 验证结果（12 + 3 = 15 项）

| # | 工具 | ChatGPT 评级 | 状态 | 备注 |
|---|---|---|---|---|
| 1 | rust-analyzer | ★★★★★ | ❌ **不可装** | `rustup component add rust-analyzer` 下载失败（cache 缺失 + 权限）|
| 2 | CodeQL | ★★★★☆ | ❌ **未装** | NVMe 空间不足；不在本地 NVMe budget |
| 3 | Semgrep | ★★★★☆ | ❌ **未装** | pip install 受限；需要 Python 环境 |
| 4 | **cargo-mutants** | ★★★★★ | ✅ **可装可跑** | v27.1.0 已在 `target/cargo-home/bin/`；已跑 sqlrustgo-transaction 全 293 mutants |
| 5 | **cargo-llvm-cov** | ★★★★★ | ✅ **可装可跑** | v0.8.7（系统 `/Users/liying/.cargo/bin/cargo-llvm-cov`）；已跑 storage + transaction 覆盖率 |
| 6 | **cargo-fuzz** | ★★★★☆ | ✅ **可装** | 已装 v0.13 到 `target/cargo-home/bin/`；但 SQLRustGo 项目当前**没有任何 fuzz target** |
| 7 | AFL++ | ★★★★☆ | ❌ **未装** | 系统级工具；非 NVMe budget |
| 8 | Sanitizers | ★★★★☆ | ❌ **需 nightly** | `rustc -Zsanitizer=address` 仅 nightly；当前 stable 1.97 拒绝 |
| 9 | cargo-deny | ★★★☆☆ | ❌ **编译失败** | v0.14 编译期需要更新的 nightly；`rmp-serde` 24 个错误，stable 无法编译 |
| 10 | cargo-semver-checks | ★★★★☆ | ❌ **编译失败** | v0.30 同样需要更新的 nightly |
| 11 | Joern | ★★☆☆☆ | ❌ **未装** | 不在 NVMe budget |
| 12 | AOCI（项目内置）| — | ✅ **可装可跑** | 4,981 entries 已建索引；`aoci.txt` 文件 7 行 manifest |
| 13 | SQLancer | 优先引入 | ⚠️ **partial** | 仓库内 `crates/sqlancer` 有适配器骨架但未跑过完整 TLP/NoREC/DQP |
| 14 | sqllogictest-rs | 重要 | ⚠️ **partial** | 仓库内 `crates/sqlrustgo_sqllogictest` 有 harness 但 corpus 复用 IAIA-410 A04 #5226 |
| 15 | cargo-flamegraph | 重要 | ❌ **未装** | system 工具；NVMe budget 受限 |

## 2. 可用工具（2 / 15）

| 工具 | 已用范围 | 待用范围（按 ChatGPT 优先级）|
|---|---|---|
| cargo-mutants | ✅ Phase 1A' 跑过 sqlrustgo-transaction 全 293 mutants | Phase 2: sqlrustgo-storage 全 7,815 mutants（**当前 worktree 正在跑**）|
| cargo-llvm-cov | ✅ Phase 1B 跑过 storage + transaction 覆盖率 | Phase 2: executor / parser / planner / optimizer 覆盖率（与本 worktree 的 mutation 跑并行）|

## 3. 不可用原因（13 / 15）

- **rust-analyzer / Sanitizers**：rustup 组件下载 cache 不可写 + 系统工具链只 stable 1.97。需 nightly 或 root 权限。
- **cargo-deny / cargo-semver-checks**：依赖 `rmp-serde` / 类似 crate，需要 nightly rustc 编译。
- **CodeQL / Semgrep / AFL++ / Joern / cargo-flamegraph**：NVMe 安装代价超出本 session budget。
- **AOCI 数据库 schema volume 缺失**（per AGENTS.md aoci 维护块）：`volume_states.database` 在状态报告中标记为 `absent`，本 worktree 的 `aoci_status.json` 与 `aoci.code.txt` 不覆盖数据库 schema，仅覆盖文件层 4981 entries。

## 4. 不在本报告做

- ❌ 跑 rust-analyzer / CodeQL / Semgrep 任何分析（工具不可用）
- ❌ 装 cargo-deny / cargo-semver-checks（编译失败）
- ❌ 改 SQLRustGo 项目内任何代码（per ADR-014 + 用户"不修改生产源码"硬约束）
- ❌ 写 SQLancer / sqllogictest 适配器（不是我的 lane，按 IAIA-410 §8 #8 移交）

## 5. ADR-014 5 字段

- **source_agent**: deepseek-flash（DeepSeek-Harness）
- **source_run**: audit-storage-mutation-20261011
- **timestamp**: 2026-10-11T07:05Z
- **evidence_hash**: `target/cargo-home/bin/cargo-mutants --help`（v27.1.0）+ `target/cargo-home/bin/cargo-fuzz --help`（v0.13）+ `cargo-deny` 编译失败 stderr（stash at /var/folders/tr/...）+ cargo metadata 验证。
- **conflict_resolution**: 不动 SQLancer / sqllogictest 适配（不属于本 lane）。SQLancer 适配由 IAIA-410 §8 #8 / A04 #5226 evidence_ready 配套。
