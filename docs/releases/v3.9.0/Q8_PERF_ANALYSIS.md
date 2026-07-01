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

# Q8/Q9 性能调查分析 (2026-06-18)

## 问题
G15 SF=0.01 wire oracle 中 Q8 (8-way join) 和 Q9 (6-way join) 超 300s 超时。

## 调查方法
1. **In-process 测试** (MemoryStorage): Q8 6.2s
2. **Wire 测试** (WalStorage<FileStorage>): Q8 >575s (10min timeout)
3. **Join trace** (在 execute_single_join 加 eprintln!): 

### In-process Q8 join trace
```
[JOIN] 200000 + lineitem → 6000000 rows in 692ms  (right=60000 rows)
[JOIN] 6000000 + orders → 1500000 rows in 1019ms (right=15000 rows)  ← filter
[JOIN] 1500000 + customer → 1500000 rows in 501ms (right=1500 rows)
[JOIN] 1500000 + nation → 1500000 rows in 568ms (right=25 rows)
[JOIN] 1500000 + nation → 1500000 rows in 696ms (right=25 rows)
[JOIN] 1500000 + region → 1500000 rows in 881ms (right=5 rows)
OK: 1 rows in 6.38s
```

### Wire Q8 join trace (60s timeout)
```
[JOIN] 200000 + lineitem → 6000000 rows in 2.78s
[JOIN] 6000000 + orders → 6000000 rows in 21.53s (60s timeout)
```

## 根本原因

### 1. Cartesians in 8-way join
Q8 的 FROM 顺序: `part, supplier, lineitem, orders, customer, nation n1, nation n2, region`

自动重写为 chain JOIN, **保留原始顺序**:
- part (2000) × supplier (100) — **cartesian 200K** (no ON key, only `s_suppkey = l_suppkey` but lineitem not yet joined)
- × lineitem (60000) — hash on p_partkey=l_partkey → **6M** (200K × 30 matches)
- × orders (15000) — hash on l_orderkey=o_orderkey → **1.5M in-process, 6M wire**

**Wire vs in-process difference**: TPC-H 真实数据 (lineitem 1-15000, 重复) vs 我的 l_orderkey=0..60000 (唯一)。
- 真实数据: 60000 lineitem, 15000 unique l_orderkey, 4 lineitem per order → 6M×1=6M (1:1 per lineitem)
- 我的数据: 60000 unique l_orderkey, only 15000 orders → 60000/4=15000 matches → 6M / 4 = 1.5M

### 2. Chain explosion
After 6M intermediate (wire), subsequent joins are:
- × customer (1500) — 6M hash on c_nationkey → ~6M (each c_custkey 1 order)
- × nation n1 (25) — 6M hash on c_nationkey → 6M
- × nation n2 (25) — 6M hash on s_nationkey → 6M (with duplicates, up to 6M×25 = 150M)
- × region (5) — 150M hash on n_regionkey → 150M

**This is the bottleneck**: 150M+ intermediate rows before WHERE filters.

## 为什么 wire 比 in-process 慢
- 同样的 join 逻辑
- 同样的 hash join 实现 (Sprint 8)
- Wire 数据更"真实"导致中间行数大 4x → 150M+ vs 6M
- 4x 增长 → 25x 慢 (因为 join 是 O(N×M) on the large intermediate)

## 修复方案

### Option 1: Join-order 优化 (理想)
**为 Sprint 9+**: 实现 CBO 或 greedy join-order selector
- 解析 WHERE 条件构建 join graph
- 按表大小+选择度排序
- 从最小表开始 (region=5, nation=25 → customer=1500 → orders=15000 → ...)
- 预期: Q8 从 150M+ intermediate → 1.5M
- 工作量: 2-3 天 (需要 planner + executor 改动)

### Option 2: Push down filters earlier (中期)
- Sprint 5 v4 已有部分 (pre_filter_cartesian_right_table)
- 扩展: 早期应用所有单表 WHERE 过滤器
- 预期: 减少 30-50% intermediate
- 工作量: 半天

### Option 3: Materialize intermediate (短期)
- 对大 intermediate 排序 + 物化到临时表
- 重新 JOIN 走索引
- 风险: 实现复杂,需要 BTree 索引支持
- 工作量: 1-2 天

## 结论
**Q8/Q9 perf 修复需要 join-order 优化 (Option 1)**。这是 Sprint 9+ 计划范围,
不在 RC8 范围。本 PR 记录了根因分析,实际修复 follow-up。

## Reference
- Sprint 8 hash join 实现: PR #3504 (MAX/MIN fix), PR #3505 (Q8/Q9 deferral doc)
- Q8/Q9 baseline: tests/data/tpch-sf01/expected/Q{8,9}_sf01_baseline.json
