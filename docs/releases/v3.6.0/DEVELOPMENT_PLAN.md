# TPC-H 性能改进开发计划

> **版本**: v3.6.0  
> **创建日期**: 2026-05-30  
> **维护人**: hermes-z6g4  
> **目标**: 让 SQLRustGo TPC-H 在 SF10 规模下可运行且性能达标

---

## 一、问题背景

### 1.1 当前状态

| 规模 | 数据量 | 状态 | 根因 |
|------|--------|------|------|
| SF0.1 | 600K lineitem | ✅ 可运行 | — |
| SF1 | 6M lineitem | ⚠️ 慢但可运行 | 单线程 |
| SF10 | 60M lineitem | ❌ 超时/OOM | 无并行 |

### 1.2 已完成

| Issue | 改动 | 验证 |
|-------|------|------|
| #196 | Graceful Hash Join (双侧分区) | SF0.1 Q3: 5712ms→4265ms, Q10: 1118ms→602ms |

### 1.3 根因分析

```
SF10 数据规模：
- lineitem: 59M 行 × 16 列 ≈ 7.6GB 原始数据
- orders: 14.7M 行 × 9 列 ≈ 1.7GB

当前架构瓶颈：
1. storage.scan() — 单线程串行读文件，I/O bound，CPU 等待
2. hash join — 单线程 build/probe，60M 行 188秒+
3. 无流式 — 整表加载到 Vec<Vec<Value>>，RSS 峰值高
4. 结果集不裁剪 — Q10 仅需 TOP 20，但全部 materialize
```

---

## 二、改进目标

| 指标 | 当前 (SF0.1) | 目标 (SF10) |
|------|-------------|-------------|
| Q1 执行时间 | 1.5s | < 60s |
| Q3 执行时间 | 4.3s | < 120s |
| Q10 执行时间 | 0.6s | < 60s |
| 内存峰值 | ~2GB | < 20GB |
| 并发度 | 1 线程 | 16 线程 |

---

## 三、任务分解

### Issue #196 ✅ 已完成
- **改动**: `hash_inner_join_rows_graceful` — 双侧哈希分区
- **验证**: SF0.1 TPC-H Q3 5712ms→4265ms (+27%)

### Issue #197 📋 流式结果集 Iterator
- **改动**: `execute_join` 返回 `Iterator<Item=Row>` 而非 `Vec<Vec<Value>>`
- **收益**: 消除中间结果累积，内存峰值 = max(单表, 单阶段) 而非叠加
- **风险**: 中，接口改动较大
- **依赖**: 无

### Issue #198 📋 LIMIT Pushdown
- **改动**: 在 join 阶段提前截断，不 materialise 全量结果
- **收益**: Q10 等 TOP-N 查询内存/时间大幅节省
- **风险**: 低，改动局部
- **依赖**: Issue #197

### Issue #199 📋 多线程并行 Scan
- **改动**: `storage.scan()` 使用 `rayon` 并行读文件分块
- **收益**: I/O 并行化，scan 阶段 4-8× 加速
- **风险**: 中，需确保线程安全
- **依赖**: 无

### Issue #200 📋 SIMD 向量化 Filter/Aggregate
- **改动**: `std::simd` 或 `portable_simd` crate，向量化列操作
- **收益**: filter/aggregate 2-4× 加速
- **风险**: 高，需要 nightly 或外部 crate
- **依赖**: Issue #199

### Issue #201 📋 TPC-H 回归基线测试
- **改动**: `scripts/gate/check_tpch.sh` — 每次 PR 必须通过 SF0.1
- **收益**: 防止性能退化
- **风险**: 无
- **依赖**: 至少 Issue #196+#199 完成

---

## 四、实现计划

### Phase 1: 内存优化（1-2周）
```
Week 1: Issue #197 流式 Iterator + Issue #198 LIMIT Pushdown
Week 2: Issue #196 的 spill-to-disk 完善
```

### Phase 2: 并行化（2-3周）
```
Week 3-4: Issue #199 多线程 scan
Week 5:   Issue #201 回归测试 + CI 集成
```

### Phase 3: SIMD 加速（4-6周）
```
Week 6-8: Issue #200 SIMD filter/aggregate
Week 9:   综合调优
```

---

## 五、验收标准

- SF10 Q1/Q3/Q10 在 10 分钟内完成，无 OOM
- SF0.1 22 个 TPC-H 查询全部通过
- 每次 PR 必须通过 `scripts/gate/check_tpch.sh`
- Clippy 零警告，Format 检查通过
- 代码覆盖率不下降

---

## 六、已知风险

1. **SF10 裸机内存**: 60M lineitem × 16 列 × ~50 字节 ≈ 48GB 原始数据，加上 build hash 和 intermediate results，即使分区也需要 20GB+ 内存
2. **SIMD nightly**: `std::simd` 需要 Rust nightly，`portable_simd` 需要引入外部依赖
3. **Iterator 接口改动**: 影响范围大，需要全量回归测试