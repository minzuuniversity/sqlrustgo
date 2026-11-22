# SQLRustGo 测试指南

## 测试目录结构

```
tests/
├── data/              # 测试数据文件 (JSON)
├── e2e/               # 端到端测试
├── integration/       # 模块集成测试
├── stress/            # 压力测试
├── ci/                # CI 专用测试
├── unit/              # 独立单元测试
├── data_loader.rs     # 统一数据加载
└── README.md
```

## 测试类型

| 类型 | 目录 | 说明 |
|------|------|------|
| 单元测试 | `crates/*/src` | 模块内 `#[cfg(test)]` |
| 模块集成 | `tests/integration/` | 模块间调用测试 |
| E2E | `tests/e2e/` | 完整查询流程测试 |
| 压力测试 | `tests/stress/` | 并发、崩溃恢复等 |
| CI | `tests/ci/` | CI 环境专用测试 |

## 运行测试

```bash
# 单元测试
cargo test --lib

# 集成测试
cargo test --test '*'

# E2E 测试
cargo test --test e2e_*

# 压力测试
cargo test --test stress_*

# 所有测试
cargo test
```

## 数据加载

使用 `tests/data_loader.rs`:

```rust
use crate::data_loader::TestDataLoader;

let data = TestDataLoader::load_json("test_data.json");
```

## NOT-RUN 披露通道（#5503）

某些目标会因为外部工具/夹具不可得而**没有真的运行**（NOT-RUN 三态，而不是通过）。
这类目标会留下一条机器可读记录：

```
TPCH-NOT-RUN: <target> reason=<原因> detail=<细节>
```

`cargo test` 会**吞掉通过测试的 stdout**，所以本地要看这些记录，请读**文件通道**：

| 场景 | 记录文件 |
|---|---|
| CI（`.gitea/workflows/ci.yml` 设置 `TPCH_NOT_RUN_FILE`） | `TPCH_NOT_RUN_FILE` 指向的文件（CI 每轮一个） |
| 本地（未设置该变量） | **默认路径**：`<target>/tpch_not_run.log`（`target` 为 cargo 的 target 目录；未设置 `CARGO_TARGET_DIR` 时是工作区的 `target/`） |

本地默认文件里保存的是**当前这一轮**的记录：第一轮测试二进制会清空它，同一轮的后续
测试二进制追加写入（轮次由 cargo 进程的 pid 标识）。若环境里没有 `ps`（无法判定轮次），
文件只追加、不清空 —— 宁可多留一条旧记录，也不丢记录。

CI 的统计口径与此一致：

```bash
NOT_RUN=$(grep -o 'TPCH-NOT-RUN:.*' "$TPCH_NOT_RUN_FILE" | sort -u | wc -l)
```

本地自证：`cargo test -p sqlrustgo --test not_run_log_visibility_5503`。
