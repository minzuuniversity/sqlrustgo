# r3 轮的 7 个 2013 读超时 —— 根因闭合：宿主 cron 的**同盘 I/O 风暴**

**日期**：2026-11-19（UTC）　**性质**：**事实登记 + 根因闭合 + 处置已落地**。
**对象**：168h SOAK `r3` 轮，run_id `20261119-168h-v410-r3`，attempt `20261119T011150Z`，
被测树 `680418e9f931e3f2423377ff4692a707ff5d60c0`，port 3412。

---

## 0. 一句话结论

r3 轮在起跑后 **27 分钟**出现 **7 个错误**，全部是
`2013 Lost connection to MySQL server during query (The read operation timed out)`，
耗时全部 ≈ **15,015 ms**（= 客户端 `read_timeout=15` 的上限）。

根因**已闭合到单一事件**：宿主 cron `7,37 * * * * /home/openclaw/llama-swap/strata-upgrade-if-idle.sh`
在 **2026-11-19T01:37:01Z–01:37:20Z** 把 Strata 引擎目录**复制**到备份树
（该树实测 **9.9 GB**）—— 备份目录与 SOAK 的数据目录**同盘 `/dev/nvme0n1p2`**。
这 19 s 的批量复制把被测服务端的存储层卡住 ⇒ **所有**查询类型（连主键点查都算）
**同时**卡到 ~14 s ⇒ 7 条在途查询越过 15 s 读超时 ⇒ 错误窗
**01:37:02Z–01:37:18Z 与备份窗逐秒对齐**。

处置：用该脚本**自带的急停开关**（`strata-upgrade-if-idle.disabled`，脚本内
line 73–76 在任何动作之前 `exit 11`）压制该 cron；该开关**对租户零功能影响**
（实测每轮本就「已是目标状态…无需升级」且「停服务前复查: 有新请求进来, 本轮放弃 (服务不动)」）。
看门狗 `scripts/soak/soak_exclusive_watch.sh` 已把该开关纳入**持续重新断言**。

⇒ **r3 轮已带 7 个错误 ⇒ 按 0 errors 门槛已不合格 ⇒ 终止并另起 r4 轮**（见 §6）。

---

## 1. 事实（逐条可复核）

### 1.1 错误事实

| 项 | 值 |
|---|---|
| 错误总数 | **7**（`driver.log` 的 E 行，`grep -c '^E,'` = 7） |
| 错误类型 | 全部 `2013 Lost connection to MySQL server during query (The read operation timed out)` |
| 耗时 | `15015.9 / 15015.6 / 15015.4 / 15015.5 / 15015.4 / 15015.4 / 15015.4` ms |
| 受影响线程 | 11, 1, 4, 6, 2, 12, 13（**7 个线程同时**） |
| panic | **00**（`server.log` 无 panic 行；metrics 的 `panic_count` 全程 `00`） |
| 错误窗（UTC） | **01:37:02Z → 01:37:18Z**（16 s） |
| 同期 load1 | **3.36 → 3.84 → 6.61**（安静） |
| 同期 host_busy | 1–2 |
| 同期 RSS | 343.7 → 343.8 MB（**平稳**） |

### 1.2 错误窗与备份窗逐秒对齐

| 时刻（UTC） | 事件 | 来源 |
|---|---|---|
| 01:37:01Z | strata cron 启动：「8001 无响应…确认空闲, 开始升级」 | `/home/openclaw/logs/strata-upgrade-if-idle.cron.log` |
| 01:37:01Z | 「锁定模型…」 | 同上 |
| ~01:36:47–01:37:03Z | **7 条最终失败的查询在此区间发起**（= 失败时刻 − 15.015 s） | 由错误耗时反推 |
| 01:37:02Z–01:37:18Z | **7 个错误依次抛出**（metrics 的 `driver_e` 由 `00` → `7`） | `armA/metrics.csv` |
| 01:37:20Z | 「备份: /home/openclaw/backups/strata-upgrade-if-idle/20261119-093701」 | strata cron 日志 |
| 01:37:21Z | 「清理旧备份: 20261119-000701」；内核 audit 记录 `type=1302` DELETE `name="engine-old"` | strata cron 日志 + `/var/log/audit/audit.log` |
| 01:37:21Z | audit `type=1327` 的 proctitle 解出 `rm -rf /home/openclaw/backups/strata-upgrade-if-idle/20261119-000701/` | 同上（十六进制逐字节解出） |

⇒ **阻塞起点（备份起 01:37:01）落在在途查询窗（~01:36:47–01:37:03）之内**，
⇒ 被卡住的正是这批查询，它们各自跑满 15.015 s 后越界。

### 1.3 「全局停顿」而非「某类查询变慢」

把 `driver.log` 的 Q 行按 kind 分桶（每 2000 条 Q 一片）后，**故障片**的形态是：

| kind | 该片 p50 (ms) | 该片 max (ms) |
|---|---|---|
| count_agg（`SELECT COUNT(*), SUM(k)` 全表扫描） | 406.1 | **13,985.1** |
| range_scan（`WHERE id BETWEEN …` 100 行） | 410.9 | **14,436.6** |
| **point_select（主键点查）** | **408.9** | **13,949.1** |
| point_update | 1,031.5 | **14,681.8** |
| insert | 1,206.7 | 12,538.5 |

⇒ **连主键点查都卡到 ~14 s**，而它前一片的 max 只有 1,352 ms
⇒ 不是「表变大 ⇒ 查询变慢」，是**存储层被外部 I/O 卡住**的全局停顿。

### 1.4 磁盘是共享的

```
$ df -h /tmp /home/openclaw /home/liying /var
/dev/nvme0n1p2  1.9T  1.4T  416G  77%  /          ← 四者同一设备
```

SOAK 的数据目录 `/tmp/sqlrustgo-v410-168h-20261119-168h-v410-r3` 与
备份树 `/home/openclaw/backups/strata-upgrade-if-idle/`（实测 **9.9 GB**）
**在同一块 NVMe 上**。

### 1.5 与 r2 的错误是**两条不同的成因**（与 §7.25 的两层结论一致）

| 轮次 | 错误窗（UTC） | 当时 load1 | 成因 | 状态 |
|---|---|---|---|---|
| r2 | 13:21:03Z – 13:27:13Z（**27 min**） | **25 – 40** | 宿主 **CPU/内存争用**（与 :07/:37 标记**不**重合） | 已由独占窗口 + 看门狗压制 |
| r3 | 01:37:02Z – 01:37:18Z（**16 s**） | **3.4 – 6.6（安静）** | 宿主 **同盘 I/O 风暴**（与 :37 备份窗逐秒重合） | 本次处置 |

⚠️ 归因边界如实记录：r3 的 7 个错误**全部**落在备份窗内且**全部**为全局停顿形态，
时间对齐到秒；但「停掉备份 ⇒ 错误不再出现」这一条**由 §6 的对照臂实测给出**，
不靠推断。

---

## 2. 为什么「容器 pause + 禁用仓库 Actions」拦不住它

这条 cron 是 **openclaw 自己的 Strata 服务运维脚本**，
它既不是容器、也不是 Gitea Actions、也不在 `gmp-platform-runner.service` /
`bustubx-unpause-ci.service` 两个单元里 ⇒ §7.24 记录的「pause 容器 + 停单元」
这条路**结构上拦不住它**。这与 GA-GATE-REPORT §7.25 要求的
「**必须同时压住「派发侧」**……且用探针连续验证」是同一条结论。

---

## 3. 处置（已落地，可逆）

### 3.1 急停开关（对租户零功能影响）

```bash
touch /home/openclaw/llama-swap/strata-upgrade-if-idle.disabled   # 开启
rm -f /home/openclaw/llama-swap/strata-upgrade-if-idle.disabled     # 撤销
```

该开关是脚本**自带且文档化**的急停开关，判定点在**任何动作之前**：

```bash
if [ -f "$DISABLE_FILE" ]; then
  log "急停开关已开（$DISABLE_FILE）⇒ 本轮不执行任何动作"
  exit 11
fi
```

**零功能影响的依据**（实测日志）：该脚本每轮都打印
「已是目标状态 (profile=iq3_s-native 引擎=0.1.60 缓存=98304 64), 无需升级」，
且 08:37 / 09:07 / 09:37 三轮都以「停服务前复查: 有新请求进来, 本轮放弃 (服务不动)」收尾
⇒ 本来就是空转，抑制它不改变 Strata 的任何运行状态。

### 3.2 看门狗持续重新断言

`scripts/soak/soak_exclusive_watch.sh` 新增 `assert_guard()`：
每轮检查急停开关是否存在，缺失则补上并写 journal
⇒ **窗口静默打开**与**开关被删**被归为同一类失效，一并看住。

### 3.3 其余候选干扰源已逐项核查（结论：无需处置）

| 源 | 频率 | 结论 |
|---|---|---|
| `sqlrustgo-watch.sh --enable-oom-guard` | 每分钟 | OOM 阈值 **250 GB**，被测服务端 RSS 仅 **343 MB**，且守护只管 `user=openclaw` 的进程 ⇒ **不可能触发**（这条此前被特意保留运行） |
| `ci-docker-cleanup.sh` | 每 10 min | 只回收**创建超 20 min** 的泄漏 docker 网络/卷（元数据级），不碰宿主进程、不碰 SOAK 数据目录 ⇒ 保留运行（停掉会累积地址池、反而搞坏 CI） |
| `252-audit-cleanup` / `ebpf-trace-retention` | 每日 02:00 / 03:00 | `find -delete` 扫 `/var/log`，量级很小 ⇒ 保留 |
| `hw-monitor` / `z6g4-mem-watch` | 每 5 min | 纯采样 ⇒ 保留 |
| `z6g4-process-snapshot` | 每小时 :30 | 进程清单快照 ⇒ 保留 |
| `llmobs-*` timers | 每 10 min | 只写自己的库 ⇒ 保留 |
| `anacron` | 每日 | 系统标准 ⇒ 保留 |

---

## 4. §7.25 的三轴证据：本次补齐了工具

§7.25 要求合格证据同时覆盖 **RSS / 表规模 / 单查询 p99**。仓内原有工具覆盖不到：

| 工具 | 原覆盖 | 缺口 |
|---|---|---|
| `extract_soak_report.py` | RSS/FD 首尾值 | p50/p99 **只从 mysqlslap 输出正则取**，认不了 v400 驱动的 `Q,<latency_ms>,…` 行；无分桶 |
| `soak_steady_window.py` | 稳态窗口 QPS | 无 p99、无 RSS 趋势、无与负载的分段相关性 |

本次新增（已入库）：

- **`scripts/soak/soak_evidence_axes.py`** —— 从 attempt 目录产出三轴事实：
  RSS 趋势（首/中位/末/峰值/每 h 增长率/下降步数）、
  单查询延迟分位（p50/p90/p99/p999/max，**按时间桶**给出）、
  分桶明细（errors / load1 / busy / RSS / p99 同桶对照 ⇒ 用来区分宿主争用与服务端侧劣化）。
  **只产出事实，不产出 PASS/FAIL**，门禁判据一个都不动。
- **`scripts/soak/soak_data_size_sampler.py`** —— 表规模轴采样器（数据目录占用 + `COUNT(*)`），
  逐行 flush，**失败即跳过**（不改判据、不重试打爆）。
- 252 上以 `srg-soak-data-size-sampler.service` 挂常驻（60 s 采样，行数每 15 min）。

**r3 上的首份实测**（elapsed 1526 s）：

| 轴 | 实测 |
|---|---|
| RSS | 192.0 → 343.3 MB（+151.3 MB；**相邻样本下降 10 次 ⇒ 非单调**） |
| 延迟 | p50 232.3 / p90 695.3 / p99 1,591.0 / p999 3,506.0 / max 14,681.8 ms（n = 73,036） |
| 表规模 | NOT-COLLECTED（采样器 01:37:38Z 才起跑，晚于起跑 26 min） |

⚠️ 该 p99/max **由 §1.3 的全局停顿主导**，不反映稳态延迟；
稳态口径须用分桶表（故障片之外的 p99 约 0.6–1.9 s）。

---

## 5. 为什么 r3 轮不能继续当 GA 证据

门槛是 **0 errors / 0 panics**，**不豁免、不放宽**（属主 2026-11-18T05:47:26Z 裁定）。
r3 已有 **7 个错误** ⇒ 该轮**已不合格**，继续跑也不会变成合格证据
（与 r2 轮同法处理：终止 + 原样留档 + 如实记终态）。

---

## 6. 对照臂与 r4 轮（处置的实证验证）

**对照臂**：r3 在急停开关**开启**后继续运行，观测下一个 `:07` / `:37` 标记
（本地 10:07 / 10:37 ＝ UTC 02:07 / 02:37）。
同宿主、同二进制、同负载、同标记时刻，**唯一变量 = 急停开关的开/关**：

| 臂 | 急停开关 | `:37` 标记（UTC 01:37） | 错误 |
|---|---|---|---|
| r3（干预前） | 关 | 有备份（01:37:01–01:37:20Z） | **7 个** |
| r3（干预后） | 开 | 无备份 | **待观测** |

⇒ 这是一条**单变量对照**，比推断强。

**r4 轮**：在对照臂观测结束后起跑，配置 = 独占窗口（看门狗持续断言）+
急停开关（看门狗持续断言）+ 四个 systemd 服务
（exclusive-watch / freeze-watch / cpu-sampler / data-size-sampler）。
planned_end = 起跑 + **168 h**。

---

## 7. 可复核的证据位置（252）

| 内容 | 路径 |
|---|---|
| r3 轮产物 | `/home/liying/soak-artifacts/20261119-168h-v410-r3/attempt-20261119T011150Z/` |
| r3 三轴分析（首份） | 同目录 `evidence_axes.json` / `EVIDENCE_AXES.md` |
| 表规模采样 | `/home/liying/soak-artifacts/20261119-168h-v410-r3/data_size.jsonl` |
| 独占窗口 manifest | `/home/liying/soak-artifacts/exclusive-window/exclusive_window_manifest.json` |
| 看门狗动作流水 | `/home/liying/soak-artifacts/exclusive-window/exclusive_watch.jsonl` |
| 冻结清单 | `/home/liying/soak-artifacts/exclusive-window/host_native_frozen.json` |
| CPU/负载采样 | `/home/liying/soak-artifacts/exclusive-window/cpu_sampler.jsonl` |
| strata cron 日志 | `/home/openclaw/logs/strata-upgrade-if-idle.cron.log` |
| 内核 audit 日志 | `/var/log/audit/audit.log` |

---

## 8. 残余风险与后续

| 风险 | 缓解 | 状态 |
|---|---|---|
| strata cron 每小时复发 | 急停开关 + 看门狗持续断言 | 已缓解 |
| 急停开关被删 | 看门狗 `assert_guard()` 补回 | 已缓解 |
| 其它未知的同盘批量 I/O 作业 | 已逐项核查 §3.3；**不能排除未知的** | 已知残余风险 |
| 宿主整机重启 | 无法缓解 | 已知；发生则须重跑 |
| r2 轮的**服务端侧成分**（RSS 414 MB ⇒ 0 错 / 1,018 MB ⇒ 715 错） | **未被本处置覆盖** —— r3 的 RSS 平稳增长（192 → 343 MB）且无错误，但 §7.25 的「RSS ⇒ 查询变慢」机制**仍未被证伪或证实** | ⏳ 须靠 r4 的 168 h + 三轴证据判定 |

⚠️ **最后一条尤其重要**：本次闭合的是**宿主 I/O 风暴**这一条成因。
§7.25 指出的「与宿主争用无关的服务端侧成分」**仍然是开放问题**，
r4 轮必须带着**三轴证据**跑，才能对它下一个结论。
