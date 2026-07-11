<!-- 2026-07-01 status addendum (auto-applied) -->
> **状态更新**: 本机 L1 lint + 架构整理已闭环。HEAD `d77821f6d1`, 3 个 PR 已合并 (PR #3664, #3665, #3666)。
> - `src/execution_engine.rs` 1471 行 (AD-001 1500 目标达标, 2630 → 1471)
> - C-ARCH-05 上限锁回 1500 (从 3000/1800 统一)
> - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
> - Open issues (4, 全部硬件阻塞, 本机无法推进):
>   - #3648 TPC-H 混合负载 SOAK 跨平台验证 (需要 Z6G4/Z440)
>   - #3423 TPC-H SF=1.0 baseline (需要 75GB+ 磁盘, Mac mini 仅 1GB)
>   - #3265 72h 长跑 SOAK (blocked-on-S1, 需 72+ 小时持续运行)
>   - #3266 168h 长跑 SOAK (blocked-on-S1, 需 168 小时持续运行)
> - 详见: issue #3667 (closed as state snapshot) + CHANGELOG.md
>
> 本文件原始内容保持不变,仅顶部加 addendum。

---

# 09 - CI Build Log

## CI Infrastructure

- **Gitea Actions**: B-Gate, E-Gate, Evidence-Graph-Gate-v4.1
- **Nomad Runners**: devstack-runner-1/2/3/4
- **Self-healing**: docker restart: always + crontab @reboot scripts

## Recent Builds (Last 7 days)

| Build | Date | Commit | Result |
|-------|------|--------|--------|
| Gitea #3370 (ODUK bugfixes) | 2026-06-13 | `c88ebc412` | ✅ PASS |
| Gitea #3369 (GA doc correction) | 2026-06-13 | `7f4ad55ae` | ✅ PASS |
| Gitea #3366 (250 G11 partial sync) | 2026-06-12 | `32dd7fb7b` | ✅ PASS |
| Gitea #3359 (Z6G4 QPS baseline) | 2026-06-12 | `87d753b10` | ✅ PASS |
| Gitea #3362 (INT-2/3 substance) | 2026-06-12 | `1210b23c9` | ✅ PASS |
| Gitea #3361 (Upgrade chain) | 2026-06-12 | `0a0dd4961` | ✅ PASS |
| Gitea #3363 (MariaDB comparison) | 2026-06-12 | `e13b26efa` | ✅ PASS |
| Gitea #3364 (Track 3 TPC-H bench) | 2026-06-12 | `f2662215` | ✅ PASS |

## Build Reproducibility

```bash
# Reproducible build command
git clone https://github.com/minzuuniversity/sqlrustgo.git
cd sqlrustgo
git checkout v3.9.0-rc7
# Latest commits post-rc7: #3370 ODUK bugfix, #3375 final 4-remote sync, #3377 .gitattributes (LFS), #3378 REMOTE_LIMITS.md
cargo build --release
cargo test --all-features
```

## CI Job Status

- **B-Gate**: ✅ PASS (last 30 runs)
- **E-Gate**: ✅ PASS (last 30 runs)
- **Evidence-Graph-Gate-v4.1**: ✅ PASS (last 30 runs)
- **Hermes Pipeline**: ✅ PASS (last 30 runs)

## CI Failure Analysis

- **0 critical CI failures** in last 7 days
- **3 transient failures** recovered automatically
- **Average CI time**: 12 minutes (build + test)
- **Average test count**: 330+

## CI Artifacts

- Binary: `target/release/sqlrustgo-mysql-server` (~12MB)
- Tests: `target/debug/deps/` (incremental)
- Logs: Gitea Actions UI
- Evidence: PR comments + commit messages
