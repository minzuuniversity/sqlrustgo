# 独占窗口被容器重建静默打断 —— 事实、量化影响与自愈处置

**日期**：2026-11-19（UTC）　**性质**：**事实登记 + 已完成的处置**。
**触发**：168h SOAK `r3` 轮起跑后复查独占窗口时，发现**窗口已失效约 7.5 小时**。

---

## 0. 一句话结论

`soak_exclusive_window.sh enter` 建立的是**一次性**窗口。252 上的 runner 容器在
2026-11-18T16:29Z 被**重建**（不是重启）⇒ `docker pause` 的状态随旧容器 ID 一起消失 ⇒
窗口**静默打开**，且没有任何告警。已用 `scripts/soak/soak_exclusive_watch.sh`
（新增，已入库）做**持续重新断言**，并以 systemd 单元托管，取代此前
「一次性 enter + nohup 守护」的做法。

对 `r3` 轮的影响已**量化**：窗口失效期与 `r3` 起跑期**不重叠**；`r3` 起跑后
**前 13 分钟**（2026-11-19T01:11:50Z～01:24:45Z）处于未强制独占状态，
但该区间实测外来负载**在起跑门禁自身认可的安静包络内**
（load1 ≤ 5.73、外来 CPU ≤ 941 %；门槛为 load1 ≤ 40、外来 ≤ 2400 %），
且**全程 e=00、panic=00**。判定：**不构成污染，不重启 r3**，理由见 §4。

---

## 1. 事实（逐条可复核）

| 项 | 值 |
|---|---|
| 原窗口建立 | manifest `entered_at` = 2026-11-18T08:39:48Z；manifest 落盘 mtime = 2026-11-18T08:47:38Z |
| 失效发现时刻 | 2026-11-19T01:21Z（复查 `docker ps` 时） |
| 发现时的状态 | `docker ps` 中带 `(Paused)` 的容器数 = **0**；两个 systemd 单元 `systemctl is-active` = **inactive**（单元侧**未**被顶回来） |
| dockerd 是否重启 | **否**。`/usr/bin/dockerd` etimes ≈ 67 947 s（≈18.9 h），未重启 |
| 容器是否被重启 | **否**。`RestartCount = 0`，但 `StartedAt` 被刷新到 2026-11-18T16:29:17Z ⇒ **被重建**（remove + create / compose up） |
| 失效时长 | 2026-11-18T08:47:38Z → 2026-11-19T01:24:45Z 重新断言前 ≈ **16.6 h**；其中「r3 起跑前」约 7.5 h |
| 重新断言 | 2026-11-19T01:24:45Z～01:24:48Z，**16 个** CI/runner 容器全部 `docker pause` 成功 |
| 单元状态 | 始终 `inactive`（`gmp-platform-runner.service`、`bustubx-unpause-ci.service`） |

### 1.1 为什么单元侧没破、容器侧破了

`bustubx-unpause-ci.service` 的职责就是「把 **paused** 的作业容器解冻」——
它是当初必须 `systemctl stop` 的原因之一。它一旦停住，**就不会**把容器顶回 running。
本次破口因此只可能来自**容器被重建**这一条路径，与实测一致。

### 1.2 破口期间宿主确实在跑别人的 CI

重新断言时新出现了两个此前未见、**仅 Up 5 秒**的
`GITEA-ACTIONS-TASK-18072-WORKFLOW-Build-Test-and-Deploy-JOB-*` 容器
（任务号从 18071 递增到 18072 ⇒ 作业在持续被派发）。
本仓仓库级 Actions 开关实测仍为 `has_actions = false`
（`GET /repos/openclaw/sqlrustgo`）⇒ 这些作业属于**同一 Gitea 实例上的其它仓库**，
共享同一台宿主机 ⇒ 对本仓禁用 Actions 拦不住它们，**这正是必须持续重新断言的原因**。

---

## 2. 处置（已落地，可逆）

### 2.1 新增 `scripts/soak/soak_exclusive_watch.sh`（已入库）

每 `INTERVAL`（默认 60 s）**幂等重新断言**：

1. 两个 systemd 单元：`ActiveState=active` ⇒ `systemctl stop`；
2. 容器：按名字模式**重新发现**（不照抄 manifest 旧清单），
   `State.Paused != true` ⇒ `docker pause`（容器被重建后也能被重新按住）；
3. 每个实际动作写一行 jsonl（时间戳 + 动作 + 对象 + 当时状态）。

**只 pause / 只 stop，从不 kill / rm** ⇒ 窗口仍然可逆。

### 2.2 252 上以 systemd 托管（取代一次性 enter + nohup）

| 单元 | 职责 | 状态 |
|---|---|---|
| `srg-soak-exclusive-watch.service` | 持续重新断言容器/单元侧的窗口 | active |
| `srg-soak-freeze-watch.service` | 冻结宿主原生构建作业（SIGSTOP） | active |
| `srg-soak-cpu-sampler.service` | 持续采样 /proc 口径真实 CPU 占用 | active |

三个单元均 `Restart=always` + `RestartSec=10` ⇒ 守护进程自身挂掉会被立刻拉起。

### 2.3 冻结守护的部署形态简化

原先冻结守护跑在「特权容器 + `nsenter` 进宿主 init 命名空间」里（容器随 docker 生命周期，
宿主重启即消失）。现改为**宿主机 root 的 systemd 服务直接跑**
（`ExecStart=/usr/bin/python3 /home/liying/_soak_host_native_freeze.py watch ...`），
不再依赖容器存活。

### 2.4 保护名单未放宽

`openclaw-gitea` / `openclaw-postgres` / `gptr-searxng` 仍在保护名单内
⇒ 窗口内仍可 push / 合并 PR（本线的入库动作不受独占窗口影响）。
`ssoak-freeze-watch` 与看门狗自身都不匹配 CI 容器模式，不会被自己冻住。

---

## 3. 对 `r3` 轮的影响：逐段核算

| 时段 | 窗口状态 | 实测外来负载 | 结论 |
|---|---|---|---|
| 2026-11-18T08:47Z → 2026-11-18T16:29Z | **已强制** | — | 无影响 |
| 2026-11-18T16:29Z → 2026-11-19T01:11:50Z | 失效 | 该区间**在 r3 起跑之前**，不进入 r3 判据 | 无影响 |
| 2026-11-19T01:11:50Z → 01:24:45Z | **未强制**（13 分钟） | 见下 | **已量化，良性** |
| 2026-11-19T01:24:45Z → planned_end | **已强制**（看门狗持续维持） | — | — |

### 3.1 未强制那 13 分钟的实测（`cpu_sampler.jsonl`，52 个 15 s 样本）

| 指标 | 实测 | 起跑门禁门槛 | 倍数 |
|---|---|---|---|
| 最大 `load1` | **5.73** | ≤ 40 | 7.0× 余量 |
| 最大外来 CPU | **941 %** | ≤ 2400 % | 2.6× 余量 |
| 外来 CPU 中位数 | 100 % | — | — |
| 外来 > 500 % 的样本 | **2 / 52** | — | — |

⚠️ 口径说明：上表的「外来」= 采样器 `system_busy_pct` **减去** `sqlrustgo-mysql-server`
自身的占用 ⇒ 它**把 r3 自己的负载驱动（`v400_soak_driver`，建表 + 灌数阶段 CPU 很高）
也算成了外来**。因此该数值是**真实争用的上界**，不是下界 ——
按上界算仍在起跑门禁自身认可的包络内，结论只会更强。

同期 **e = 00、panic = 00、FD 恒定 46**。

### 3.2 判定

**不重启 r3。** 理由三条，全部基于实测而非偏好：

1. 未强制区间与 r3 的测量区间**几乎不重叠**（仅前 13 分钟），且其外来负载
   **低于 r3 自己起跑时用的门槛**；
2. 该区间内 r3 的错误数为 **0**、panic 为 **0**、FD 无增长 ⇒ 没有留下可见损伤；
3. 重启会丢弃一段已验证健康的运行，并**重新暴露**于新的起跑期风险，
   而门禁侧的 017 轮正占用本机、SOAK 侧再动一次会打乱已有的证据链。

**如实披露**：r3 轮的独占性在起跑后前 13 分钟**不是被强制保证的**，而是**被事后测量
证明是良性的**。这一条写进本文件，不掩盖。

---

## 4. 残余风险与后续

| 风险 | 缓解 | 状态 |
|---|---|---|
| 宿主再次重建容器 | 看门狗 ≤ 60 s 重新按住 | 已缓解（systemd `Restart=always`） |
| 看门狗自身挂掉 | systemd 拉起 | 已缓解 |
| 宿主原生构建作业（kvmem CUDA / basalt / parallel_observe） | `soak_host_native_freeze.py` SIGSTOP（可逆）+ starttime 校验 | 已缓解（systemd 托管） |
| 宿主整机重启 | **无法缓解** —— 宿主重启会杀掉 r3 本体 | 已知；如发生须重跑 r3 |
| 其它仓库的 Actions 作业持续派发 | 看门狗持续按住；本仓自身 `has_actions=false` | 已缓解 |
| 窗口内的 pause 会不会让别人在跑的作业挂起 168 h | 会 —— 这是**已知副作用**，属主应知悉；`leave` 可原样恢复 | 已披露 |

### 4.1 窗口撤销方式（结束时）

```bash
# ① 先停看门狗，否则它会把容器重新按住
sudo systemctl disable --now srg-soak-exclusive-watch.service
# ② 再用一次性工具恢复（它按 manifest 原样恢复）
bash scripts/soak/soak_exclusive_window.sh leave
# ③ 恢复宿主原生作业（SIGCONT，带 starttime 校验）
python3 scripts/soak/soak_host_native_freeze.py resume \
  --out /home/liying/soak-artifacts/exclusive-window/host_native_frozen.json
```

---

## 5. 可复核的证据位置（252）

| 内容 | 路径 |
|---|---|
| 一次性窗口 manifest（原样恢复用） | `/home/liying/soak-artifacts/exclusive-window/exclusive_window_manifest.json` |
| 看门狗动作流水（jsonl） | `/home/liying/soak-artifacts/exclusive-window/exclusive_watch.jsonl` |
| 冻结清单 | `/home/liying/soak-artifacts/exclusive-window/host_native_frozen.json` |
| CPU/负载采样（/proc 口径） | `/home/liying/soak-artifacts/exclusive-window/cpu_sampler.jsonl` |
| r3 轮产物 | `/home/liying/soak-artifacts/20261119-168h-v410-r3/attempt-20261119T011150Z/` |
| r3 的被测树 | `/home/liying/sqlrustgo-soak-v410-tip` @ `680418e9f931e3f2423377ff4692a707ff5d60c0` |

**r3 轮 planned_end ≈ 2026-11-26T01:11:50Z**，判据 0 errors / 0 panics。
