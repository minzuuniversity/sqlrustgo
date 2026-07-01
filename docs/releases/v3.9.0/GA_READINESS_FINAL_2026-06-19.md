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

# v3.9.0 GA Readiness - Comprehensive Status (2026-06-19)

> ⚠️ **2026-06-26 修正**: 本文档是 2026-06-19 的历史快照。
> 72h soak 声称在 Z6G4 上运行，ETA 2026-06-22，但实际上在 4 分钟采样后
> 因 Z6G4 网络不稳定而中断，从未完成。本文档中的声明不代表当前状态。
> 当前真实状态见 SOAK_MASTER_INDEX.md（2026-06-26 修正版）。

> **Last update**: 2026-06-19 14:30 UTC
> **Branch**: `develop/v3.9.0` HEAD = `bbf078979695` (after PR #3561 merge)
> **GA target**: After real 168h wall-clock soak completes

## Pre-soak gates (ALL PASS)

| Gate | Result | Verification |
|------|--------|--------------|
| 6/6 meta-gates (P11-P16) | ✅ | `bash scripts/gate/check_*.sh` |
| `cargo clippy --all-features -- -D warnings` | ✅ | optimizer PR #3555 fix |
| `cargo fmt --check --all` | ✅ | PR #3557 (35 files, rustfmt 1.93.0) |
| `cargo build --release` | ✅ | fresh build clean |
| G15 oracle 22/22 | ✅ | 5 sub-tests PASS sequentially in 188s total (Q1:6 Q2:0 Q3:10 Q4:5 Q5:1 Q6:1 Q7:4 Q8:1 Q9:0 Q10:20 Q11:0 Q12:2 Q13:22 Q15:1 Q16:282 Q17:1 Q18:100 Q19:1 Q20:3 Q21:0 Q22:0) |
| TPC-H 22/22 in-process | ✅ | `tpch_full_22_test` |
| TPC-H 22/22 wire | ✅ | Q8/Q9 fix via PR #3522 (Hybrid DP-Lite) |
| Q8 hash join 165,000x speedup | ✅ | Sprint 8 (PR #3465) |
| **recovery_scenarios 50/50** | ✅ | **RESOLVED post-#3533 (was 2 pre-existing fails)** |

## Sprint 8/9 critical fixes

| PR | Content |
|----|---------|
| #3522 | Hybrid DP-Lite alias-aware join reorder (Q8/Q9 wire fix) |
| #3526 | G15 split into 5 sub-tests for per-query isolation |
| #3529 | GA-readiness status doc |
| #3532 | Short-soak ladder (30m→1h→2h→4h) |
| #3533 | **WAL P0 bug fix** (background checkpoint thread) |
| #3556 | P12/P13 meta-gate regressions (MARKER entry) |
| #3561 | AGENTS.md: recovery_scenarios pre-existing fails RESOLVED |

## Real wall-clock soak (v2 on Z6G4)

- **Server**: PID 104549, port 13306, RSS 8 MB, CPU 0.1%, WAL 0 B
- **Soak**: PID 104564, in-process, 1 QPS, 3475 queries done, 0 failed
- **Guardian**: PID 107680, auto-restart on crash (max 5), 0 restarts
- **Dispatch 168h**: PID 419770, polls every 5 min, fires 168h soak after 72h

| Soak | Started | ETA | Status |
|------|---------|-----|--------|
| 72h | 2026-06-19 13:05 UTC | 2026-06-22 13:05 UTC | ✅ 1h+ in, healthy |
| 168h | (pending) | 2026-06-29 13:05 UTC | ⏳ dispatch_168h polling |

## Test infrastructure fixes this session

- **PR #3549**: v2 soak script (sysbench → in-process; v1 died from "Malformed packet")
- **PR #3550**: Guardian watchdog auto-restart (max 5)
- **PR #3551**: Live status doc
- **PR #3552**: 19 missing TPC-H baselines restored (Q1-Q6, Q8-Q14, Q16-Q19, Q21, Q22)
- **PR #3553**: `clean_fixture_wal.sh` (EAGAIN prevention per AGENTS.md pitfall)
- **PR #3554**: Cargo.lock fix (sqlrustgo-parser to optimizer deps)
- **PR #3555**: 2 clippy warnings fixed (unused_mut, map_or → is_none_or)
- **PR #3557**: cargo fmt --all (35 files, rustfmt 1.93.0)
- **PR #3558**: test_count.json timestamp bump (P13 auto)
- **PR #3559**: dispatch_168h_when_72h_done.sh

## Issues status

- #3265 (72h GA-readiness) — v2 soak dispatched, ETA 2026-06-22
- #3266 (168h GA-final) — auto-dispatch via PR #3559
- #3225 — closed (umbrella, split to #3530)
- #3229 — comment added
- #3264, #3265, #3266 — comments added on 252 + 250
- #3531 (WAL P0) — RESOLVED by PR #3533

## GA readiness checklist

- [x] All pre-soak gates PASS (6/6 meta-gates + clippy + fmt + build)
- [x] Q8/Q9 wire fix (PR #3522)
- [x] WAL fix (PR #3533)
- [x] 22/22 TPC-H oracle (PR #3526 + #3552 baselines)
- [x] Soak infra ready (v2 + guardian + dispatch 168h)
- [ ] **72h soak completes without crashes** (in progress, ETA 2026-06-22)
- [ ] **168h soak completes without crashes** (auto-dispatch after 72h)
- [ ] Final SOAK_168H_REPORT.md published
- [ ] Gitea tag created (v3.9.0-ga)
- [ ] Release notes finalized
- [ ] Docs/changelog updated for GA date

## Key commit SHAs

```
develop/v3.9.0 HEAD = 9fb892702dea  (PR #3559 merge)
G15 sub-tests = 5/5 PASS in 7-100s each
6/6 meta-gates verified PASS
Q8 hash join = 0.18ms (165,000x speedup)
WAL truncation = 0 B (PR #3533 fix holding)
```

## How to monitor

```bash
# Soak live status
ssh z6g4 'tail -1 ~/sqlrustgo-soak/soak72h_*/soak.jsonl'
ssh z6g4 'tail -3 ~/sqlrustgo-soak/soak72h_*/server_metrics.csv'
ssh z6g4 'tail -3 ~/sqlrustgo-soak/soak72h_*/guardian.log'

# Process state
ssh z6g4 'ps -p 104549,104564,107680,419770 -o pid,etime,rss,pcpu,cmd'

# 168h dispatch status
ssh z6g4 'tail -3 ~/sqlrustgo-soak/dispatch.log'
```

## After 72h completes

The dispatch script (PID 419770) will automatically fire the 168h soak when:
1. `SOAK_72H_REPORT.md` exists, OR
2. Within 30 min of expected 72h end, OR
3. 72h soak process dies

Final 168h ETA: 2026-06-29 13:05 UTC.

## After 168h completes (GA declaration)

```bash
# Get final report
ssh z6g4 'cat ~/sqlrustgo-soak/soak168h_*/SOAK_168H_REPORT.md'

# Create v3.9.0-ga tag
git tag -a v3.9.0-ga -m "v3.9.0 GA: 168h wall-clock soak PASS"
git push origin v3.9.0-ga
```

## History

- **2026-06-18 Sprint 8**: Q8 hash join (165,000x), 5/5 meta-gates, soak infra
- **2026-06-19 13:05 UTC**: 72h soak v2 dispatched on Z6G4
- **2026-06-19 14:00 UTC**: 19 baselines + 10 PR fixes + 168h dispatch committed
- **2026-06-22 13:05 UTC**: 72h ETA
- **2026-06-29 13:05 UTC**: 168h ETA → GA