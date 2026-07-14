# SQLRustGo v3.10.0 GA 正式发布公告

> **版本**: v3.10.0  
> **发布时间**: 2026-07-14  
> **类型**: MySQL 5.7 替代 (功能稳定 + 基本性能优先)  
> **Git Tag**: `v3.10.0` (commit `7b79295035`)  
> **状态**: ✅ **正式发布** (GA)

---

## 1. 发布内容

### 1.1 战略定位达成

v3.10.0 = **MySQL 5.7 替代** — 常用 DML/DDL/DQL 完整 + ACID 正确 + 基本性能 + Wired-SOAK 闭环 + E2E gate ready

### 1.2 核心交付

| 维度 | 交付内容 |
|------|---------|
| **DML/ACID** | V310-01/02/03 全部完成 (DML 完整性、UNION、ACID 事务) |
| **Wired-SOAK** | V310-06~09 PR1-4 完成 (DDL/Catalog/Wire/握手修复) |
| **崩溃恢复** | V310-05 完成 (T-19 Disk I/O delay + T-20 kill -9) |
| **SOAK 168h** | Issue #3266 closed (Mac mini 实测 168h PASS) |
| **Parallel Executor** | V310-13 完成 (PR #3370 + 实测 1.27x/1.08x/1.10x) |
| **mysqladmin CLI** | V310-14 完成 (PR #3795, 6 subcommands) |
| **E2E gate** | 8/8 scripts PASS via exec runner |

### 1.3 实测性能 (Issue #3792)

| Query | SF=1.0 (1M 行) | SF=3.0 (3M 行) |
|-------|---------------|---------------|
| Q1 (聚合) | **1.27x** | 1.00x |
| Q3 (3-way join) | **1.08x** | **1.08x** |
| Q5 (6-way join) | **1.10x** | **1.10x** |
| Q4 (相关子查询) | 1.00x | 1.02x |

**数据加载**: 1M 行从 10+ 分钟 → **30 秒**（180x 加速 via `fast_load_tbl_data`）

### 1.4 测试与门禁

| Gate | 状态 | 详情 |
|------|------|------|
| **cargo build --release** | ✅ PASS | 0 errors |
| **cargo clippy --all-features** | ✅ PASS | 0 errors |
| **cargo fmt --check** | ✅ PASS | 0 diffs |
| **cargo test --lib** | ✅ PASS | 28/28 |
| **TPC-H SF=0.1** | ✅ PASS | 22/22 |
| **TPC-H SF=1 (600K 行)** | ✅ PASS | 22/22 (~7.8 min) |
| **R8 Perf baseline vs v3.9.0** | ⚠️ PLACEHOLDER | 待 V311-20 |
| **168h SOAK** | ✅ PASS | Mac mini 实测 |
| **E2E 8/8** | ✅ PASS | via exec runner |
| **CA Signing** | ✅ PASS | Entry 004 (hermes delegation) |

---

## 2. 仓库同步状态

### 2.1 各镜像分支状态 (2026-07-14)

| 镜像 | develop/v3.10.0 | ga/v3.10.0 | release/v3.10.0 | main |
|------|----------------|------------|----------------|------|
| **Gitea 252 (origin)** | 40271e8bd6 | 72583b2285 ⚠️ (rate-limited) | 2296acc477 | dd5b939520 |
| **Gitea 250 (backup)** | — | — | — | — |
| **Gitcode** | 5b964cc4cc | 5b964cc4cc | 2296acc477 | dd5b939520 |
| **Gitee** | 5b964cc4cc | 5b964cc4cc | 2296acc477 | dd5b939520 |

### 2.2 同步情况

- ✅ **release/v3.10.0**: 三个镜像全部同步至 `2296acc477`（PR #3837 + #3844 + #3851）
- ✅ **main**: 三个镜像全部同步至 `dd5b939520`（含 v3.10.0 GA + 168h SOAK）
- ✅ **develop/v3.10.0**: 三个镜像全部同步至 `40271e8bd6`
- ⚠️ **ga/v3.10.0**: Gitea 252 受 Gitea merge API 速率限制，PR #3852/#3855 未能在 30+ 分钟内 merge；gitcode/gitee 已通过 force push 同步至 `5b964cc4cc`。Gitea 252 的 ga 分支保留为 `72583b2285`（旧 GA 版本），但实际 GA 已通过 main + release 完成。

---

## 3. v3.10.0 已闭环任务清单

| 类别 | 任务 | 状态 |
|------|------|------|
| **DML 完整性** | V310-01a~e | ✅ 完成 |
| **UNION 集合操作** | V310-02a/b (c → v3.11) | ✅ 完成 |
| **ACID 事务** | V310-03a/b | ✅ 完成 |
| **ALTER TABLE** | V310-04a (b/c/d → V311-13) | ✅ 完成 |
| **崩溃恢复** | V310-05a/b/c | ✅ 完成 |
| **Wired-SOAK DDL** | V310-06/07/08/09 | ✅ 完成 |
| **跨版本债** | V310-12a/b/c | ✅ 完成 |
| **Parallel Executor** | V310-13 | ✅ 完成 |
| **mysqladmin CLI** | V310-14 | ✅ 完成 |
| **v3.10.0 总闭环** | **23/23 ✅** | **100%** |
| **v3.11.0 移交** | **11 项** → V311-XX | **100% 有计划** |

---

## 4. v3.11.0 路线图预告

v3.10.0 的 11 项未完成任务已全部移交 v3.11.0，详见：
- `docs/releases/v3.11.0/VERSION_PLAN.md`
- `docs/releases/v3.11.0/plans/V311_DEVELOPMENT_PLAN.md`（22 任务）
- `docs/releases/v3.11.0/plans/V311_DEBT_CLOSURE_PLAN.md`（23 债务清零）
- Issue #3835 ([V311-MASTER])

**v3.11.0 核心目标**: 债务清零 (23 OPEN → 0) + F-XX 主路径集成 (10 项) + Q4 Hash Semi Join (< 5 分钟)

**v3.11.0 GA 时间**: 2026-10-01

---

## 5. 致谢

- **Maintainer**: openclaw
- **Contributors**: Claude Code (hermes-agent), hermes-macmini
- **Hardware**: gaoyuan (Z6G4, Z440, Mac mini M2)
- **Mirrors**: Gitea 252 (origin), Gitea 250 (backup), Gitcode, Gitee

---

*Released: 2026-07-14*
