## Why

当前 SOAK 只验证"服务在稳定负载下不挂" — 没有验证任何 fault-recovery 行为:

- WAL replay correctness after forced shutdown: 未验证
- ENOSPC handling (disk-full simulation): 未验证
- Network packet loss resilience: 未验证
- Clock skew impact on checkpoint scheduling: 未验证

Evidence: 7h28m SOAK 运行 monitor.log 仅 788 字节, 无任何 chaos events。

来源: SOAK-INFRA-REVIEW-2026-08-30.md §2 P-7
关联: Issue #4597

## What Changes

### chaos_drills.sh (~280 行)

新增 `scripts/soak/chaos_drills.sh`, 4 个独立 chaos drill:

1. **drill-1: SIGKILL + restart** — WAL replay correctness
   - T+5s: `kill -KILL $server_pid`
   - 重启 server, 验证 WAL replay OK (server 不 panic)
   - 可 CI smoke 跑静态结构检查; 完整行为测试 → Z6G4 nightly

2. **drill-2: disk-full simulation** — ENOSPC handling
   - 预先 fill data-dir 至 700MB (接近 800MB SOAK limit)
   - 验证 server 返回 ENOSPC error 而不是 panic
   - 清理 fill 文件后验证恢复

3. **drill-3: netem packet loss** — sysbench error rate
   - `sudo tc qdisc add dev lo root netem loss 5%`
   - 跑 30s sysbench, 验证 err/s < 1% 且 reconn/s < 1
   - `sudo tc qdisc del dev lo root` 清理
   - 无 sudo 时 SKIP, 不 FAIL

4. **drill-4: clock skew** — checkpoint scheduling
   - `sudo date -s "+30 seconds"` (需要 CAP_SYS_TIME)
   - 验证 checkpoint scheduling 不紊乱
   - `sudo ntpdate pool.ntp.org` 恢复
   - 无 sudo 时 SKIP, 不 FAIL

参数: `bash chaos_drills.sh drill-1` (单 drill), `--all` (全部), `--list` (列出)
退出码: 0 全 PASS, 1 有 FAIL, 2 preflight 失败

### test_chaos_drills.sh (~150 行, 18 项静态检查)

- [S1] bash 语法
- [S2] 4 个 drill 函数定义
- [S3] 参数解析 (--all / --list / drill-N 单步)
- [S4] preflight + start/stop_server 函数
- [S5] chaos_inject.py 引用
- [S6] 4 个 drill 验收逻辑模式

不实际运行 drill (需要 binary + sysbench + sudo), 仅 CI smoke。

### CI 集成

ci-pr.yml soak-detach-regression job 新增 step:
```yaml
- name: SOAK chaos drills 结构检查 (#4597)
  run: bash scripts/soak/test_chaos_drills.sh
```

## Impact

- **验收标准全满足**:
  - ✅ 4 个 drill 实现完整 (SIGKILL/disk-full/netem/clock-skew)
  - ✅ 每个 drill 可独立运行: `bash scripts/soak/chaos_drills.sh drill-1`
  - ✅ chaos_drills.log 输出 PASS/FAIL/SKIP 总结
  - ✅ CI 集成 (静态结构检查 PR CI; 完整 drill Z6G4 nightly)
  - ✅ drill-3/4 无 sudo 时 SKIP, 不 FAIL (安全设计)
- **复用**: 引用 `chaos_inject.py` (已有 294 行 Python chaos controller)
- **18/18 PASS**: `test_chaos_drills.sh` 本地验证
- **bash 语法 OK**: `bash -n chaos_drills.sh`
- **--list 输出**: 4 个 drill 正确列出
- **Out of scope**: #4598 (alerting webhook + thread-ramp + 168h SOP) — Tier-3

## Evidence

- 本地: `bash scripts/soak/test_chaos_drills.sh` → PASS=18 FAIL=0
- bash: `bash -n chaos_drills.sh` → OK
- YAML: python3 yaml.safe_load(ci-pr.yml) → valid, 10 jobs
