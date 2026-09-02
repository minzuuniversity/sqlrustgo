## Why

SOAK-INFRA-REVIEW-2026-08-30.md §2 P-5/P-6/P-10 指出 3 个 Tier-2 缺口:

1. **P-5 disk-restart boundary**: restart_count 是 local 变量, 跨 disk-limit / PID-death 重启后 metrics.csv header 无 `restart_seq` 列, 无法把 `metrics.csv.0/1/2/...` 拼回完整 QPS 时间线
2. **P-6 workload sweep**: sysbench 硬编码 `oltp_read_write`, 无法切换 `oltp_read_only` / `oltp_write_only` / `oltp_insert` / `oltp_update_index` 跑 sweep 比较
3. **P-10 periodic_reports JSON**: periodic_reports.log 是人类可读格式, 无法直接被 Prometheus / Grafana / jq 消费 (需正则解析时间/RSS/QPS)

## What Changes

### P-6 SOAK_WORKLOAD env var (~5 行)

- 新增 `SOAK_WORKLOAD="${SOAK_WORKLOAD:-oltp_read_write}"`
- `start_sysbench()` prepare/run 命令把 `sysbench oltp_read_write` 改为 `sysbench "${SOAK_WORKLOAD}"`
- 文档注释列出 5 种 workload: oltp_read_write / oltp_read_only / oltp_write_only / oltp_insert / oltp_update_index

### P-10 periodic_reports.jsonl JSON 输出 (~10 行)

- `setup_run_dir()` 新增 `PERIODIC_JSONL="${RUN_DIR}/periodic_reports.jsonl"`
- `record_metrics()` JSON line `printf` 输出:
  ```
  {"ts":"2026-08-30T14:30:00+0800","epoch":1788283800,"elapsed_s":3600,"rss_mb":123.4,"fd":12,"threads":8,"wal_mb":45.67,"disk_mb":300,"server_qps":200,"sysbench_qps":180,"workload":"oltp_read_write","restart_seq":0}
  ```
- 12 字段: ts/epoch/elapsed_s/rss_mb/fd/threads/wal_mb/disk_mb/server_qps/sysbench_qps/workload/restart_seq
- 保留 human-readable periodic_reports.log (不删)

### P-5 disk-restart boundary (~20 行)

- `local restart_count=0` → 全局 `RESTART_SEQ=0` (供 `record_metrics()` + JSON 引用)
- metrics.csv header: 新增 `restart_seq` 列 → `ts,elapsed_s,...,restart_seq`
- `record_metrics()` CSV 行追加 `RESTART_SEQ` 值
- disk-limit 触发重启 / PID 死亡重启: `cp metrics.csv metrics.csv.$((RESTART_SEQ-1))` 切分
- progress display 改用 `restart=${RESTART_SEQ}`
- grep 验证 active code 无 `restart_count` 残留

### test_run_soak_loop_tier2.sh (~150 行)

17 项静态检查: SOAK_WORKLOAD 替换 / workload 文档 / JSONL 12 字段 / RESTART_SEQ 全局化 / header 列 / csv 行 / 切分行为 / progress 显示 / 零残留 等。

### CI 集成

ci-pr.yml `soak-detach-regression` job 新增 step:
```yaml
- name: SOAK Tier-2 workload sweep + JSON reports + disk-restart boundary (P-6/P-10/P-5)
  run: bash scripts/soak/test_run_soak_loop_tier2.sh
```

## Impact

- **验收标准全满足**:
  - ✅ SOAK_WORKLOAD=oltp_read_only 可选, start_sysbench() 无 `oltp_read_write` 硬编码
  - ✅ periodic_reports.jsonl 可用 jq 解析 (12 字段 JSON line)
  - ✅ 跨 restart_cycle 能拼回时间线: metrics.csv.<N> + restart_seq 列 + PERIODIC_JSONL.restart_seq
- **零回归**: #4595 test_run_soak_loop_detach.sh (8/8) + tier1.sh (15/15) 继续 PASS
- **17/17 PASS**: `bash scripts/soak/test_run_soak_loop_tier2.sh`
- **bash 语法 OK**: `bash -n run_soak_loop.sh`
- **YAML valid**: 10 jobs (python3 yaml.safe_load)
- **Out of scope**: P-11 webhook / P-8 thread-ramp / P-12 SOP (#4598 Tier-3)

## Evidence

- 本地: `test_run_soak_loop_tier2.sh` → PASS=17 FAIL=0
- 回归: `test_run_soak_loop_detach.sh` → PASS=8 FAIL=0
- 回归: `test_run_soak_loop_tier1.sh` → PASS=15 FAIL=0
- bash 语法: `bash -n run_soak_loop.sh` → OK
- 残留检查: `grep restart_count run_soak_loop.sh` → 0 matches
- YAML: python3 yaml.safe_load → valid, 10 jobs

关联: Issue #4596 / SOAK-INFRA-REVIEW-2026-08-30.md §2 P-5/P-6/P-10
