# SOAK 168h Local Runbook (v312-59-D / #4598 P-12)

> 168 小时本地长跑 SOAK 的标准操作流程
> 适用版本: develop/v3.12.0
> 适用硬件: HP Z6 G4 (`hp-z6g4` runner, 192.168.0.252)
> 关联脚本: `scripts/soak/run_soak_loop.sh`、`scripts/soak/chaos_drills.sh`

---

## 1. 适用范围与目标

本 SOP 描述如何在 hp-z6g4 自托管 runner 上跑一次 168 小时（7 天）本地 SOAK 测试。
168h 是 v3.12.0 GA 前的标准长跑门槛，目的是在长时间运行下暴露：

- **内存泄漏**（RSS 单调增长 / 7d 后 OOM）
- **WAL 膨胀**（未 checkpoint 的脏页累积）
- **FD 泄漏**（connection 未释放导致 fd 耗尽）
- **线程数漂移**（线程池未正确回收）
- **QPS 缓慢退化**（buffer pool 抖动 / plan cache 污染）

168h SOAK 不替代自动化 CI 中已有的 SOAK gate（`docs/gates/`），
其目的为**事后取证**与**每周 release readiness 信号**。

> v312-59-D / #4598 P-8 另提供 `SOAK_THREAD_RAMP="1 2 4 8 16 32"` 线程斜坡模式
> （`run_thread_ramp`），用于在 < 1h 内找到 QPS 饱和点，不必等满 168h。
> v312-59-D / #4598 P-11 提供 `SOAK_ALERT_WEBHOOK` 阈值告警（RSS / QPS drop），
> 详见 §3.2 与 §4.4。

---

## 2. 硬件前置

### 2.1 hp-z6g4 自托管 runner

- CPU: Intel Xeon Gold 6248R @ 3.0GHz (24c/48t)
- RAM: 128 GB DDR4-2933 ECC
- 磁盘: NVMe SSD 2TB（`/home` mountpoint，**避免 tmpfs**）
- OS: Linux 7.0.0-30-generic
- 网络: 1GbE，与 Gitea 同主机直连
- 自托管 runner 标签: `hp-z6g4`，详见 `.gitea/workflows/ci.yml`

### 2.2 二进制构建

168h SOAK 必须用 `--release` 二进制：

```bash
cd /home/openclaw/workspace/dev/sqlrustgo
git checkout develop/v3.12.0
git pull --ff-only
cargo build --release -p sqlrustgo-mysql-server
ls -la target/release/sqlrustgo-mysql-server
# 期望: -rwxr-xr-x ... 50-80 MB
```

### 2.3 端口与磁盘

| 项目 | 默认 | 推荐覆盖 | 理由 |
|------|------|---------|------|
| `SOAK_PORT` | 3396 | `3396` | 避开主服务端口 |
| `SOAK_DATA_DIR` | `~/sqlrustgo-soak-data-${PORT}` | `/home/sqlrustgo-soak-data` | **必须非 tmpfs**，否则 `start_server` 检测后会警告并跨重启丢数据 |
| `SOAK_RESULTS_DIR` | `~/sqlrustgo-soak-results` | `/home/sqlrustgo-soak-results` | 落盘需 ≥ 5 GB 空闲（CSV/log/JSONL 单 run ~3 GB × 多个 cycle） |
| `SOAK_HOURS` | 24 | `168` | 本 SOP 目标 |
| `SOAK_WORKLOAD` | `oltp_read_write` | 同左 | 168h 默认场景 |

> ⚠️ **不要**把 `SOAK_DATA_DIR` 设在 `/tmp`（tmpfs），`start_server:294-296` 已有警告，
> 但手动 `SOAK_DATA_DIR=/tmp/...` 覆盖后会绕过检测。

---

## 3. 启动流程

### 3.1 端口 + 进程预检

```bash
# 1. 端口空闲
lsof -i :3396 -sTCP:LISTEN || echo "port 3396 free"

# 2. sysbench 已装
sysbench --version | head -1
# 期望: sysbench 1.0.20 (或更新)

# 3. lsof / curl / jq 可选 (webhook 需 curl, JSON payload 需 jq)
command -v lsof && command -v curl && command -v jq

# 4. 没有任何残留 SOAK 进程
pgrep -fa 'run_soak_loop\|sqlrustgo-mysql-server\|sysbench' || echo "clean"
```

### 3.2 配置可选 webhook 告警（v312-59-D P-11）

```bash
# Slack incoming webhook 例子 (URL 仅作示意, 实际值由运维提供)
export SOAK_ALERT_WEBHOOK='https://hooks.slack.com/services/T.../B.../...'

# 阈值覆盖 (默认 RSS>500MB, QPS drop>50%)
export SOAK_ALERT_RSS_MB=600
export SOAK_ALERT_QPS_DROP_PCT=40
export SOAK_ALERT_CURL_TIMEOUT=5

# 若不设 SOAK_ALERT_WEBHOOK, 告警仅写 MONITOR_LOG (grep ALERT 即可)
```

### 3.3 启动命令

```bash
cd /home/openclaw/workspace/dev/sqlrustgo

# 168h 跑
SOAK_HOURS=168 \
  bash scripts/soak/run_soak_loop.sh
```

脚本会以非 TTY 模式自动走 `setsid+nohup+disown`（参见 `run_soak_loop.sh:104-136`），
产生 `~/sqlrustgo-soak-results/launcher_<ts>.pid` 与 `launcher_<ts>.log`。

**手动覆盖**（用于交互式终端或调试）：

```bash
SOAK_HOURS=168 SOAK_NO_DETACH=1 bash scripts/soak/run_soak_loop.sh
```

### 3.4 确认启动

```bash
# PID 文件
cat ~/sqlrustgo-soak-results/launcher_*.pid  # launcher shell PID
cat ~/sqlrustgo-soak-results/soak_*/server.pid  # sqlrustgo 实际 PID

# 进程在
pgrep -fa 'sqlrustgo-mysql-server'  # server PID
pgrep -fa 'sysbench.*--test=oltp_read_write'  # sysbench PID

# latest_run.txt
cat ~/sqlrustgo-soak-results/latest_run.txt
```

### 3.5 1 小时内烟雾验证

启动后等 60-90 分钟，让 server 跑稳、sysbench 报告 ≥ 6 轮（`--report-interval=10`）：

```bash
# latest 10min report
tail -30 ~/sqlrustgo-soak-results/soak_*/periodic_reports.log

# QPS > 0 且非 NaN
jq -c 'select(.sysbench_qps > 0)' \
   ~/sqlrustgo-soak-results/soak_*/periodic_reports.jsonl | tail -3

# server 无 panic
grep -ciE 'panicked|fatal runtime error' \
   ~/sqlrustgo-soak-results/soak_*/server.log
# 期望: 0

# 若 RSS > 800 MB 或 sysbench_qps 持续 < 50, 立即排查 (见 §6)
```

---

## 4. 监控与日常巡检

### 4.1 实时观察

```bash
# 当前 run_dir
RUN_DIR=$(grep -E '^/' ~/sqlrustgo-soak-results/latest_run.txt | head -1)
echo "RUN_DIR=${RUN_DIR}"

# 1 行 KPI (RSS / WAL / QPS / restart)
tail -1 "${RUN_DIR}/periodic_reports.log"

# JSON 末尾
tail -1 "${RUN_DIR}/periodic_reports.jsonl" | jq .
```

### 4.2 QPS 退化信号（v312-59-D INCIDENT-2026-08-30 教训）

7h28m 的 SOAK run 中，QPS 从 193 缓慢退化到 117（39% drop），但没自动告警。
事后才在 history CSV 发现。修复在 `run_soak_loop.sh` 中加了 `send_alert`（P-11），
但仍建议每周手动检查一次：

```bash
# 24h 内的 QPS 趋势
jq -r '"\(.ts) \(.sysbench_qps)"' "${RUN_DIR}/periodic_reports.jsonl" | \
  awk '$2 > 0 {print $1, $2}' | tail -150 | \
  awk '{sum+=$2; cnt++; if(cnt==1) min=$2; min=($2<min)?$2:min; max=($2>max)?$2:max}
       END {printf "samples=%d avg=%.1f min=%.1f max=%.1f\n", cnt, sum/cnt, min, max}'
```

`avg` 与 `max` 比值 < 0.7 时启动根因排查。

### 4.3 磁盘水位

```bash
# SOAK_RESULTS_DIR 用量
du -sh ~/sqlrustgo-soak-results

# 当前 data-dir (DISK_LIMIT 800MB 触发自动 disk-restart)
du -sh ~/sqlrustgo-soak-data-${SOAK_PORT:-3396}
```

`run_soak_loop.sh:596-632` 在 `disk_b > DISK_LIMIT_BYTES (800MB)` 时自动 disk-restart，
这是预期行为。`restart_seq` 列在 metrics.csv 里随 cycle 自增，
`HISTORY_CSV` 跨 RUN_DIR 持久化（用于拼接）。

### 4.4 webhook 触达校验（如果 §3.2 设了 SOAK_ALERT_WEBHOOK）

```bash
# grep ALERT 行
grep -E '^.*ALERT \[' ~/sqlrustgo-soak-results/soak_*/monitor.log | tail -10

# 检查 webhook 是否真的发出去 (Slack/Email 收到才算)
# 这里只能间接验证: MONITOR_LOG 中 ALERT 紧跟 "alert webhook POST 失败" warn=没发出去
```

---

## 5. 停止与归档

### 5.1 优雅停止

```bash
# 拿 launcher PID
LPID=$(cat ~/sqlrustgo-soak-results/launcher_*.pid | head -1)

# SIGTERM 给 launcher shell, 它会传 trap EXIT → cleanup()
kill -TERM "${LPID}"

# 验证 30s 内退出
for i in $(seq 1 30); do
    if ! kill -0 "${LPID}" 2>/dev/null; then
        echo "launcher exited after ${i}s"
        break
    fi
    sleep 1
done

# server/sysbench 应已被 cleanup() 杀掉
pgrep -fa 'sqlrustgo-mysql-server\|sysbench.*--test=oltp_read_write' || echo "all clean"
```

### 5.2 强制停止（仅在 §5.1 超时后）

```bash
# 直接杀 server + sysbench, 然后 launcher
SPID=$(cat ~/sqlrustgo-soak-results/soak_*/server.pid | head -1)
BPID=$(cat ~/sqlrustgo-soak-results/soak_*/sysbench.pid | head -1)
kill -KILL "${SPID}" "${BPID}" 2>/dev/null || true
kill -KILL "${LPID}" 2>/dev/null || true

# 清理 data-dir (下次启动 setup_run_dir 会重新建)
rm -rf ~/sqlrustgo-soak-data-${SOAK_PORT:-3396}
```

> ⚠️ 强制停止会丢 `MONITOR_LOG` 中尚未 flush 的最后 1-2 行 periodic report。
> 168h run 中尽量只在异常（runner OOM、网络分区）时使用。

### 5.3 归档（Evidence Retention）

`docs/releases/v3.12.0/evidence/issue-4598/` 下按 run 时间戳归档：

```bash
DEST="docs/releases/v3.12.0/evidence/issue-4598/run_$(date +%Y%m%d_%H%M%S)"
mkdir -p "${DEST}"

# 复制结果 (避免硬链, 历史不应被新 run 覆盖)
cp -r ~/sqlrustgo-soak-results/soak_* "${DEST}/"
cp ~/sqlrustgo-soak-results/soak_history.csv "${DEST}/"
cp ~/sqlrustgo-soak-results/launcher_*.log "${DEST}/"
cp ~/sqlrustgo-soak-results/launcher_*.pid "${DEST}/"

# 生成 1-page summary
{
    echo "# SOAK 168h Run Summary — $(basename "${DEST}")"
    echo
    echo "## KPI"
    jq -r '"\(.ts) \(.rss_mb) \(.server_qps) \(.sysbench_qps)"' \
       "${DEST}"/*/periodic_reports.jsonl | \
       awk '$2>0 && $4>0 {n++; rss+=$2; sv+=$3; sb+=$4;
                          if(n==1){min_sb=$4; max_sb=$4}
                          min_sb=($4<min_sb)?$4:min_sb
                          max_sb=($4>max_sb)?$4:max_sb}
            END {printf "samples=%d avg_rss_mb=%.1f avg_server_qps=%.1f avg_sysbench_qps=%.1f min_sb=%.1f max_sb=%.1f\n",
                       n, rss/n, sv/n, sb/n, min_sb, max_sb}'
    echo
    echo "## Panics"
    echo "panic_count=$(grep -ciE 'panicked|fatal runtime error' "${DEST}"/*/server.log | head -1)"
    echo
    echo "## Restarts"
    jq -r '.restart_seq' "${DEST}"/*/periodic_reports.jsonl | sort -u | wc -l
} > "${DEST}/SUMMARY.md"

ls -la "${DEST}/"
```

---

## 6. 故障排查速查

| 现象 | 第一时间动作 | 进一步定位 |
|------|------------|----------|
| Server PID 消失 | `tail -50 ${RUN_DIR}/server.log` + `grep -E 'panicked\|fatal\|abort' ${RUN_DIR}/server.log` | 若 OOM killer: `dmesg \| grep -i 'killed process'`；若 panic: 看 stack trace |
| QPS 突降 ≥50%（P-11 自动告警） | `tail -3 ${RUN_DIR}/server.log ${RUN_DIR}/sysbench.log` | 检查 server 是否处于 GC/compaction；`RESOURCE_MONITOR` 行的 `total_q` 增长曲线 |
| RSS > 1GB | `awk '/VmRSS/{print $2}' /proc/$(cat ${RUN_DIR}/server.pid)/status` 多次采样 | 若单调增长: leak; 若锯齿状: buffer pool 正常 LRU 抖动 |
| WAL > 100 MB | `ls -la ${RUN_DIR%/*}/*/sqlrustgo.wal` + `grep checkpoint ${RUN_DIR}/server.log` | WAL 不缩说明 checkpoint 没跑 |
| FD > 200 | `ls /proc/$(cat ${RUN_DIR}/server.pid)/fd \| wc -l` 每 5min | 若单调增长: fd leak; 检查连接回收路径 |
| launcher 启动后立刻退出 | `cat ~/sqlrustgo-soak-results/launcher_*.log` | 大概率 port 占用或 binary 不在；exit code 在最后一行 |
| Webhook 全部失败 | `grep 'alert webhook POST 失败' ${RUN_DIR}/monitor.log` | curl `--max-time 5s` 超时 → 加大；网络断 → 用本地 file sink（待办） |
| 168h run 跑完后 metrics 缺段 | 检查 `restart_seq` 是否连续；`HISTORY_CSV` 应无 gap | 若有 gap: disk-restart 后 PERIODIC_LOG 未及时写入 |

---

## 7. 失败恢复 / 重跑决策

168h 跑崩后决策树：

```text
崩溃时间点 < 168h?
├─ 是 → 排查 §6 → 修代码/环境 → 重跑 (新 timestamp, 不续跑)
└─ 否 (跑满) → §5.3 归档 → 开新 issue (若发现 regression) → 不直接重跑
```

不建议"续跑"（从崩溃时刻 resume）的原因：
- crash → restart 之间的 WAL/binary data 一致性需重新校验
- periodic_reports.jsonl append-only 设计假设无中断
- 168h 本身就是为了暴露问题；中断后重置比续跑更接近"用户 7 天使用"场景

---

## 8. 关联文档

- `scripts/soak/run_soak_loop.sh` — 主循环（~700 行，含 P-11/P-8 实现）
- `scripts/soak/chaos_drills.sh` — 4 个 fault-recovery drill（v312-59-D #4597）
- `docs/releases/v3.12.0/evidence/issue-4560/INCIDENT-REPORT-2026-08-30.md` — 7h28m SOAK 教训
- `docs/releases/v3.12.0/evidence/issue-4560/SOAK-INFRA-REVIEW-2026-08-30.md` — P-1..P-12 整改追踪
- `docs/runbooks/z6g4-ssh-recovery.md` — 同 runner SSH 恢复
- `docs/runbooks/INDEX.md` — runbook 索引（本文档需追加条目）

---

*维护者: SQLRustGo v3.12.0 release team*
*最后更新: 2026-09-02（v312-59-D / #4598 Tier-3 落地）*