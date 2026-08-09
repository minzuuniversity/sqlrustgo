# V313-07: 修复窗口函数 RANK() OVER (PARTITION BY ...) 的 Wire Protocol 截断 Bug

## Why

V312-21 的 `window_rank_partition_unsupported` fixture 揭示了以下问题：

- **表面错误**：`RANK() OVER (PARTITION BY a, b ORDER BY c)` 触发 `"Protocol: row packet truncated: rpos=1 pkt_len=1"`
- **根因**：Wire Protocol 层在编码窗口函数结果行时发生截断，而非窗口函数执行器本身不可用
- `WindowVolcanoExecutor::compute_rank()` 已实现（RANK、DENSE_RANK 均已在单元测试中验证通过）
- 错误发生在结果集从 executor 返回后、通过网络协议层序列化的环节

本变更修复 Wire Protocol 编码器，确保 `RANK()`、`ROW_NUMBER()`、`DENSE_RANK()` 等窗口函数结果能正确序列化并返回给客户端。

## What Changes

- **`crates/mysql-server/src/`**：定位并修复窗口函数结果行的 Packet 序列化截断逻辑
- **`tests/compat/mysql_v3_13/window_rank_partition_fix.sql`** + `.out`（新增）：验证 `RANK()`、`ROW_NUMBER()`、`DENSE_RANK()` 在 `PARTITION BY` 场景下正确返回
- **`tests/compat/mysql_v3_13/window_rank_basic.sql`** + `.out`（新增）：验证无 PARTITION BY 的基础窗口函数
- **`tests/compat/mysql_v3_12/window_rank_partition_unsupported.sql`**：将 `# expect` 从 `DEFERRED` 更新为 `PASS`，正式解除 deferred 状态
- **`docs/releases/v3.13.0/evidence/mysql_compat/`**：更新 `SURFACE_DISPOSITION.md` 中 `window_rank_partition` 行

## Capabilities

### 新增能力

- `window-rank-wire-protocol`：窗口函数结果行正确通过 MySQL Wire Protocol 序列化，无截断错误
- `window-rank-partition`：支持 `RANK() OVER (PARTITION BY col1, col2 ORDER BY col3)` 的完整执行链路

### 修改能力

- `deferred-window-rank-partition`：将 V312-21 的 `DEFERRED: wire protocol truncation bug` 状态升级为 `PASS`

## Impact

- **修改文件**：`crates/mysql-server/src/` 中 Packet 序列化相关代码
- **新增文件**：
  - `tests/compat/mysql_v3_13/window_rank_partition_fix.sql` + `.out`
  - `tests/compat/mysql_v3_13/window_rank_basic.sql` + `.out`
- **修改文件**：`tests/compat/mysql_v3_12/window_rank_partition_unsupported.sql`（expect 从 DEFERRED 改为 PASS）
- **风险**：Wire Protocol 修复可能影响其他结果集编码路径；需在全面回归测试后合并
- **无新增外部 crate 依赖**
