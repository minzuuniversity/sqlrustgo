# CI 与发布门禁设计

## 触发矩阵

| 入口 | 常规回归 | 契约域 | 允许 EXPECTED_RED | 发布决定 |
|---|---:|---:|---:|---:|
| 根包 `cargo test --all-features --no-fail-fast` | 是 | 否 | 否 | 否 |
| workspace member job | 是 | 否 | 否 | 否 |
| `cargo test --manifest-path tools/contract-test-domain/Cargo.toml ...` | 否 | 是 | reproduce 模式可记录 | 否 |
| Test Factory classifier | 否 | 是 | 只分类，不转 PASS | 否 |
| release applicability gate | 引用 | 引用 | 开放项阻断 | 是 |

## 设计约束

1. 独立 package 不加入 workspace members；用负向测试确认 `cargo metadata` 的 workspace_members 不含它。
2. 常规 CI 日志由现有 `check_test_target_execution.sh --scope root/crates` 核对；契约域另以计划集合核对，不篡改现有 scope 语义。
3. `--all-features` 只能作用于所选 manifest，不能把独立包带入根包。
4. 契约 job 必须始终执行，不能依赖 `dsh-b/*` 等临时分支过滤；实际 workflow 修改属于 TF-06，需单独批准。
5. 契约 job 的 EXPECTED_RED 不等于 CI PASS：reproduce job 可完成证据采集，但汇总状态必须显示 `OPEN_DEFECTS`。
6. 发布 gate 根据阶段、Issue、ADR 条件和适用 case 计算；任何 NOT_RUN、签名漂移或未关闭适用 RED 均拒绝。

## 防绕过验收

- 从计划删除一个 case：计划签名或映射完整性失败。
- 测试 binary 未执行：实际集合少于计划集合，NOT_RUN/exit 2。
- 把失败签名改宽：mutation 测试必须检出。
- 已知 RED 突然绿：UNEXPECTED_GREEN/exit 1，要求独立复验，不能自动关 Issue。
- 常规 CI 意外执行契约包：隔离测试失败。

本文件是设计，不授权编辑 `.gitea/workflows/` 或 `scripts/gate/`。
