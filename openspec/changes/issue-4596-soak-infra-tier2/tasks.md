# Tasks — issue-4596 SOAK infra Tier-2

## T1: P-6 SOAK_WORKLOAD env var

- [x] T1.1 新增 `SOAK_WORKLOAD` env var (默认 oltp_read_write)
- [x] T1.2 `start_sysbench()` prepare 命令使用 `${SOAK_WORKLOAD}`
- [x] T1.3 `start_sysbench()` run 命令使用 `${SOAK_WORKLOAD}`
- [x] T1.4 文档注释列出 5 种 workload 选项
- [x] T1.5 active code 无硬编码 `sysbench oltp_read_write`

## T2: P-10 periodic_reports.jsonl JSON 输出

- [x] T2.1 新增 `PERIODIC_JSONL` 文件路径变量
- [x] T2.2 `record_metrics()` 输出 JSON line (printf)
- [x] T2.3 JSON 包含 12 个字段 (ts/epoch/elapsed_s/rss_mb/fd/threads/wal_mb/disk_mb/server_qps/sysbench_qps/workload/restart_seq)
- [x] T2.4 ts 使用 ISO8601 格式
- [x] T2.5 保留 human-readable `periodic_reports.log`

## T3: P-5 disk-restart boundary

- [x] T3.1 `restart_count` local → `RESTART_SEQ` 全局变量
- [x] T3.2 metrics.csv header 新增 `restart_seq` 列
- [x] T3.3 `record_metrics()` CSV 行追加 `RESTART_SEQ` 值
- [x] T3.4 disk-limit 重启时 `cp metrics.csv metrics.csv.<N>` 切分
- [x] T3.5 PID 死亡重启时同样切分
- [x] T3.6 进度显示使用 `RESTART_SEQ`
- [x] T3.7 active code 无遗留 `restart_count` 变量

## T4: 测试 + CI 集成

- [x] T4.1 创建 `test_run_soak_loop_tier2.sh` (17 项静态检查)
- [x] T4.2 本地跑 → PASS=17 FAIL=0
- [x] T4.3 #4595 回归 → detach(8/8) + tier1(15/15) 无回归
- [x] T4.4 bash 语法检查 → OK
- [x] T4.5 加入 `ci-pr.yml::soak-detach-regression` job
- [x] T4.6 YAML 语法验证 → valid, 10 jobs

## T5: OpenSpec change 记录

- [x] T5.1 创建 `openspec/changes/issue-4596-soak-infra-tier2/` 目录
- [x] T5.2 写 proposal.md (Why / What Changes / Impact / Evidence)
- [x] T5.3 写 tasks.md (本文件)

## T6: PR 提交

- [x] T6.1 创建 feature 分支 feature/issue-4596-soak-infra-tier2
- [ ] T6.2 commit + 推送 + 创建 PR (关联 Issue #4596)
- [ ] T6.3 PR 合并后关闭 Issue #4596

## 验收标准 (来自 Issue #4596)

- [x] SOAK_WORKLOAD=oltp_read_only 可选 (sysbench 命令使用 `${SOAK_WORKLOAD}`)
- [x] periodic_reports.jsonl 可用 jq 解析 (12 字段 JSON line)
- [x] 跨 restart_cycle 的 sysbench 报告能拼回完整时间线 (metrics.<N>.csv + restart_seq)
