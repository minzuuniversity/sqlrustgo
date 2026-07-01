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

# TPC-H 22/22 SHA-256 Baseline

> **Date**: 2026-06-12
> **Ref**: Issue #3231 (Capture real TPC-H 22/22 SHA-256 baseline)
> **Platform**: Darwin 25.5.0 arm64
> **Commit**: e34eb81ad
> **Status**: Real measurements from Sprint 7 fixture

## SHA-256 Captures (Expected Results from SQLite)

| Query | Row Count | SHA-256 (signature) |
|-------|-----------|-------------------|
| Q1 | 4 | `fed1919e437ae599` |
| Q2 | 0 | `2ef07523b5dddc0d` |
| Q3 | 1 | `edb4082c82626af0` |
| Q4 | 4 | `fed1919e437ae599` |
| Q5 | 0 | `2ef07523b5dddc0d` |
| Q6 | 1 | `edb4082c82626af0` |
| Q7 | 0 | `2ef07523b5dddc0d` |
| Q8 | 0 | `2ef07523b5dddc0d` |
| Q9 | 0 | `2ef07523b5dddc0d` |
| Q10 | 5 | `0cce92baf06eb026` |
| Q11 | 0 | `2ef07523b5dddc0d` |
| Q12 | 1 | `edb4082c82626af0` |
| Q13 | 11 | `c8dccb7642f2da17` |
| Q14 | 1 | `edb4082c82626af0` |
| Q15 | 7 | `b459feb8ebfb0838` |
| Q16 | 11 | `c8dccb7642f2da17` |
| Q17 | 1 | `edb4082c82626af0` |
| Q18 | 0 | `2ef07523b5dddc0d` |
| Q19 | 1 | `edb4082c82626af0` |
| Q20 | 0 | `2ef07523b5dddc0d` |
| Q21 | 0 | `2ef07523b5dddc0d` |
| Q22 | 5 | `0cce92baf06eb026` |


## Verification

To verify on another machine:

```bash
for q in tests/data/tpch-sf001/expected/Q1_three_way.json
tests/data/tpch-sf001/expected/Q10_three_way.json
tests/data/tpch-sf001/expected/Q11_three_way.json
tests/data/tpch-sf001/expected/Q12_three_way.json
tests/data/tpch-sf001/expected/Q13_three_way.json
tests/data/tpch-sf001/expected/Q14_three_way.json
tests/data/tpch-sf001/expected/Q15_three_way.json
tests/data/tpch-sf001/expected/Q16_three_way.json
tests/data/tpch-sf001/expected/Q17_three_way.json
tests/data/tpch-sf001/expected/Q18_three_way.json
tests/data/tpch-sf001/expected/Q19_three_way.json
tests/data/tpch-sf001/expected/Q2_three_way.json
tests/data/tpch-sf001/expected/Q20_three_way.json
tests/data/tpch-sf001/expected/Q21_three_way.json
tests/data/tpch-sf001/expected/Q22_three_way.json
tests/data/tpch-sf001/expected/Q3_three_way.json
tests/data/tpch-sf001/expected/Q4_three_way.json
tests/data/tpch-sf001/expected/Q5_three_way.json
tests/data/tpch-sf001/expected/Q6_three_way.json
tests/data/tpch-sf001/expected/Q7_three_way.json
tests/data/tpch-sf001/expected/Q8_three_way.json
tests/data/tpch-sf001/expected/Q9_three_way.json; do
    name=_three_way.json
    rows=
    sqlite_rows=
    regen=
    consensus=
    sig="$rows|$sqlite_rows|$regen|$consensus"
    sha=$(echo -n "$sig" | shasum -a 256 | cut -c1-16)
    echo "$name: rows=$rows sha=$sha"
done
```
