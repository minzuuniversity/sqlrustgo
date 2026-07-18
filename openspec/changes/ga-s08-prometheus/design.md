## Overview

Prometheus 指标导出和 Slow Query Log 设计文档。

## Prometheus Metrics

### `--metrics-addr` 参数
- HTTP 服务监听地址
- `/metrics` 端点暴露 Prometheus 格式指标

### 指标列表
- `sqlrustgo_qps_total` — 查询总数
- `sqlrustgo_query_duration_seconds` — 查询延迟分位数
- `sqlrustgo_connections_active` — 活跃连接数
- `sqlrustgo_buffer_pool_hit_ratio` — Buffer Pool 命中率

## Slow Query Log

### `long_query_time` 阈值
- 超过阈值的 SQL 记录到文件
- JSON 格式：query_time, rows_examined, sql_text
