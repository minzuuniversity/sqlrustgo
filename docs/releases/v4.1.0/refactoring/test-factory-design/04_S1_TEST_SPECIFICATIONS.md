# S1 三例正式规格

## TF-S1-001 / INV-08

- 当前函数：`c01_07c_paired_frontier_observation_must_exist`。
- 分类：`GAP_PROBE`，不是 Contract/Regression/Acceptance Test。
- 当前预期：接口缺失时探针观察到缺口；输出必须为 `NOT_IMPLEMENTED`，不得 PASS。
- 最小正确实现：真实提交路径暴露同一 stream/epoch 下的 commit frontier 与 durable frontier；同步 ACK 前可运行断言 `durable_lsn >= commit_lsn`。
- 正确测试：执行一次同步提交，读取该次 receipt/frontier，比较 stream、epoch 与 LSN。
- 反向注入：让 ACK 在 sync 前返回、让 durable frontier 落后、让两个 LSN 属于不同 stream；均须 RED。
- 禁止：搜索 `durable_lsn`/`commit_lsn` 字符串作为正确性证明；源码缺失后 return 成功。

## TF-S1-002 / INV-14

- 当前函数：`c01_09a_capability_descriptor_must_exist_for_write_before_refuse`。
- 分类：`GAP_PROBE`。
- 当前预期：能力描述 API 不存在时输出 `NOT_IMPLEMENTED`。
- 最小正确实现：后端返回结构化能力集；缺事务能力的后端在任何事务写入前返回 typed Unsupported。
- 正确测试：读取实际 descriptor，再调用 begin/write；descriptor 与行为必须一致。
- 反向注入：仅增加 `capabilities` 名称、谎报 transactions=true、返回 Ok 但不工作；均须 RED。
- 禁止：只匹配方法名或注释。

## TF-S1-003 / INV-14

- 当前函数：`c01_09c_wrapper_capability_attestation_must_be_observable`。
- 分类：`INEFFECTIVE_TEST`；当前固定 `None` 的断言无判别力。
- 当前预期：不得计入测试覆盖；正式状态为 `NOT_IMPLEMENTED`。
- 最小正确实现：wrapper 输出 `effective_capabilities = inner_capabilities + verified_delta` 及证据；不得凭空增加事务/undo 能力。
- 正确测试：用至少两个 inner（支持/不支持事务）装配 wrapper，验证声明与实际 begin/commit/rollback 行为一致。
- 反向注入：wrapper 虚报能力、证据缺失、inner 不支持但 wrapper 返回空成功；均须 RED。
- 禁止：局部常量、类型名、字符串或函数名存在性充当行为证明。

三个 case 的稳定 ID 为 `TF-S1-001..003`。旧函数名作为 source alias 保留在迁移映射中，不作为身份主键。
