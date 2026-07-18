## Why

当前仅验证了 SF=1 (~1GB 数据)，GA 发布前必须验证生产级扩展性。SF=10 (~10GB) 可验证 Join 重排算法在更大基数下是否仍能避免 OOM。

## What Changes

- 新增 `scripts/tpch/setup_sf10.sh` — SF=10 数据生成脚本
- 新增 `scripts/tpch/run_sf10.sh` — SF=10 测试运行脚本
- 新增 `scripts/gate/check_tpch_sf10.sh` — GA-P1 门禁脚本
- 扩展现有 TPC-H 基础设施支持 SF=10

### 测试目标

1. SF=10 数据生成 (~10GB)
2. 22/22 查询全部通过
3. 峰值 RSS 和执行时间记录
4. 无 OOM 验证

## Capabilities

### New Capabilities

- `tpch-sf10`: TPC-H SF=10 基准测试能力

### Modified Capabilities

- `tpch`: 扩展现有 TPC-H 测试支持 SF=10
