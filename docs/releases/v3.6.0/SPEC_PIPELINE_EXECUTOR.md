# SPEC: PipelineExecutor Core

> **Issue**: #1460
> **Status**: Draft
> **Created**: 2026-05-29
> **Branch**: `develop/v3.6.0`

---

## 1. 概述

### 1.1 目标

实现统一执行架构核心：`PipelineExecutor` + `Operator` trait。

### 1.2 为什么需要 PipelineExecutor

当前 SQLRustGo 有 4 套执行模型并行：
- Volcano Executor（行式）
- Parallel Executor（并行）
- Stored Proc Runtime
- Vector Search

导致：无法统一插入 SIMD/spill/exchange，测试复杂度爆炸。

### 1.3 解决方案

统一 Pipeline 模型：

```text
PipelineExecutor
    ├── Driver (消费 Chunk)
    ├── Pipeline (算子链)
    └── Operator (算子接口)
```

---

## 2. 架构设计

### 2.1 Operator Trait

```rust
/// 统一算子接口。所有执行器实现此 trait。
pub trait Operator {
    /// 初始化算子（分配资源、建立连接等）
    fn open(&mut self) -> ExecResult<()>;

    /// 消费一个 Chunk，返回下一个 Chunk 或 Done
    fn next_chunk(&mut self) -> ExecResult<ChunkOrDone>;

    /// 清理资源
    fn close(&mut self) -> ExecResult<()>;

    /// 返回算子需要的 Schema
    fn schema(&self) -> &Schema;
}
```

### 2.2 ChunkOrDone

```rust
pub enum ChunkOrDone {
    Chunk(Chunk),
    Done,
}
```

### 2.3 Pipeline

```rust
/// Pipeline 是一组按序执行的 Operator。
pub struct Pipeline {
    operators: Vec<Box<dyn Operator>>,
}

impl Pipeline {
    pub fn new() -> Self;
    pub fn add(&mut self, op: impl Operator + 'static);
    pub fn execute(&mut self) -> ExecResult<Vec<Chunk>>;
}
```

### 2.4 Driver

```rust
/// Driver 消费 Pipeline 产生的 Chunk。
pub trait Driver {
    fn process(&mut self, chunk: Chunk) -> ExecResult<()>;
    fn finalize(self) -> ExecResult<()>;
}
```

---

## 3. 迁移策略

### 3.1 Phase 1: 不破坏现有功能

- 新增 `pipeline/` 模块
- 现有 volcano/parallel/stored_proc 不变
- 通过 feature gate 控制

### 3.2 Phase 2: 适配基础算子

- SeqScan → ChunkSeqScan
- Filter → ChunkFilter

### 3.3 Phase 3: 完整迁移

- 所有算子走 Pipeline
- 删除旧适配层

---

## 4. 目录结构

```
crates/executor/src/pipeline/
├── mod.rs
├── operator.rs      # Operator trait
├── pipeline.rs     # Pipeline struct
├── driver.rs       # Driver trait
├── chunk_or_done.rs
└── adapters/       # 旧算子适配层
    └── row_to_chunk.rs
```

---

## 5. 验收条件

| 条件 | 说明 |
|------|------|
| Operator trait 定义完成 | 所有方法签名正确 |
| Pipeline execute 方法工作 | 链式执行 Chunk |
| SeqScan 适配器 | 现有 SeqScan 包装为 ChunkScan |
| Filter 适配器 | 现有 Filter 包装为 ChunkFilter |
| 编译通过 | cargo build -p sqlrustgo-executor |
| 测试通过 | 现有测试不破坏 |

---

## 6. 与其他 Issue 的关系

- #1459 (Chunk)：**依赖**，Pipeline 用 Chunk 作为数据单元
- #1461 (SIMD)：**后续**，Pipeline 是 SIMD 的载体
- #1463 (stored_proc)：**后续**，stored_proc 迁移到 Pipeline

---

## 7. 风险

| 风险 | 影响 | 缓解 |
|------|------|------|
| 破坏现有 executor | 高 | Phase 1 用 feature gate，不改变现有代码 |
| Schema 兼容性 | 中 | Operator.schema() 提供类型信息 |

---

*Spec version: 1.0 | 待实现*
