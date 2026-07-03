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

# 08 - License Compliance

## License Summary

| Component | License | Compatible |
|-----------|---------|------------|
| SQLRustGo source code | Apache-2.0 OR MIT | ✅ (permissive) |
| All workspace crates | Apache-2.0 OR MIT | ✅ |
| All dependencies (top 50) | MIT/Apache-2.0/BSD/ISC | ✅ |
| No GPL/AGPL/LGPL deps | n/a | ✅ |

## License Verification

```bash
# Verify no copyleft licenses in deps
cargo deny check license
# Result: 0 violations
```

## Source Files

All first-party code in this repository:
- Dual-licensed under Apache-2.0 OR MIT
- Contributors retain copyright
- See `LICENSE-APACHE` and `LICENSE-MIT` for full text

## Dependencies (sample top 10)

| Crate | License |
|-------|---------|
| tokio | MIT |
| rustls | Apache-2.0/ISC/MIT |
| serde | Apache-2.0/MIT |
| clap | Apache-2.0/MIT |
| chrono | Apache-2.0/MIT |
| bincode | MIT |
| crc32fast | Apache-2.0/MIT |
| zstd | MIT |
| lz4 | Apache-2.0/MIT |
| uuid | Apache-2.0/MIT |

All MIT/Apache-2.0/BSD-compatible. No GPL contamination.

## Third-party Code

No third-party code copied into source. All functionality implemented from scratch.

## Trademark

"SQLRustGo" is a project name, not a registered trademark. No trademark concerns.
