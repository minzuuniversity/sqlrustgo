# GA_RELEASE_REPORT.md — v4.1.0 GA 发布报告

> **本报告的结论是「未发布」。**
> 下面逐条写清：哪些判据已用实跑证据闭合、哪些还红、红在哪、消除条件是什么。
> 任何没有实跑证据的项，本报告一律不写 PASS。

- 版本：v4.1.0
- 报告日期：2026-11-16
- 冻结提交（本次全部实跑的唯一基线）：`7e26c2b7c6840e3bbec804a0bf3bbcc706a646d7`
- 阶段（实测）：**ALPHA**（`docs/releases/v4.1.0/STAGE.yaml` → `current_stage`）
- 配套文件：`GA_GATE_REPORT.md`（门禁快照）、`STAGE_GOVERNANCE_RC_TO_GA.md`（阶段治理与签批）、
  `GATE_BASELINE_V410.jsonl`（168 门禁归属基线）

---

## 0. 一句话结论

**v4.1.0 今日不可发布为 GA。** 原因不是「代码没写完」，而是四条**外部/物理/人类**约束
同时未解除（见 §3）。本轮已把**仓内可由 AI 完成的部分全部做完并归档**，
并把通往 GA 的 168h SOAK **今天启动**（最早 2026-11-23 满 168h）。

## 1. 本轮做了什么（可复核）

| # | 动作 | 证据 |
|---|---|---|
| 1 | **GA 门禁基线自有化重盘**：4.0.0 继承清单 → v4.1.0 自有基线（判据继承、清单重盘） | `docs/releases/v4.1.0/GATE_BASELINE_V410.jsonl`（168 行）；生成器 `audit/v4.1.0/gate_baseline_v410.py`（12 臂反向变异全绿）；PR #5804 |
| 2 | **两处存量红清零**：`cargo fmt --check --all` 10 处 → rc=0；`check_commit_message.sh` 1 条 BAD_SHAPE → rc=0（根因是类型表不完整，按 2026-11-08 补 `audit` 的同型先例补表） | PR #5804；`check_commit_message.sh --selftest` rc=0；`test_criteria_scope_and_types.sh` rc=0 |
| 3 | **#5598 两处 fail-open 闭合**（退出码三态化 0/1/2）＋ A7-4 测量假阴性修正 | PR #5805；fx06/fx09/fx10 三道负向夹具实测全部 FIXTURE-PASS |
| 4 | **168h SOAK 运行器交付并启动**（两臂：服务端稳定性 + 多模型 W1..W5） | PR #5806；`scripts/soak/run_v410_168h_soak.sh`；GA-2 harness 时长可配（`SQLRUSTGO_SOAK_DURATION_SECS`，单测 1 passed） |
| 5 | **GA 全清单 35 项实跑**（冻结提交 `7e26c2b7c6`） | run `20261116-ga-gates-002`（plan sha256 `sha256:6466a0ec156…`） |

## 2. GA 判据逐条状态（rc_to_ga）

| 判据 | 状态 | 依据 |
|---|---|---|
| All RC{N} release-blocking bugs closed | **未验证** | 仓内无执行点；本轮未做 RC 候选轮次 |
| All 5 dimensions (D1-D5) PASS | **实跑中** | run `20261116-ga-gates-002`；结果以该 run 的 `results.jsonl` 为准 |
| 168h multi-model SOAK PASS | **未通过**（今日物理不可达） | 本轮 2026-11-16 启动两臂 run；满 168h 最早 2026-11-23 |
| Tag v4.1.0-rcN cut, no P0 bugs since | **未验证** | 无 rc tag；P0 计数无门禁覆盖 |
| GA promotion approved by human architect | **未取得** | `transitions.RC_to_GA.requires_human_approval: true`；明文「批准记录不能由 Agent 生成」 |
| Coverage >= 80% | **实跑中** | 同 run |
| GA 门禁清单 35 项全绿 | **实跑中** | 同 run |
| GA 承载分支（main / release/v*） | **未满足** | `release/v4.1.0` 尚不存在；develop 上不得宣称 GA |

## 3. 四条硬约束（AI 不可自行消除）

1. **SOAK-168H**：物理需 7 天独占窗口。本轮已启动 ⇒ 最早 2026-11-23。
2. **HUMAN-SIGNOFF**：阶段跃迁签批须人类架构师在 `STAGE_GOVERNANCE_RC_TO_GA.md` §7 署名。
3. **CARRIER**：GA 须 `main` / `release/v*` 承载（`carrier_enforced: true`）。
4. **BASELINE-GAP**：168 道门禁重盘后，**49 道「应在范围内执行但当前无执行点」**；
   owner 2026-11-08 裁决要恢复的 23 道**至今未落地**（`gov_probe` 实测 `dead` 仍 = 61）。

另有两条**外部**约束不在 AI 权限内：
- **Hub #133 独立 Reviewer 回执未回** ⇒ 阶段跃迁证据链缺第 5 环；
- **镜像漂移**：gitea250 / gitcode / gitee 落后 develop 若干合并；
  `AGENTS.md` §3.2 明文「只 gitea252 可写；gitcode/gitee/github 为只读镜像，**禁止 push**」
  ⇒ 同步留给属主或同步脚本。

## 4. 本轮明确**没有**做的事（如实登记）

- **没有**宣布任何阶段跃迁（`current_stage` 仍为 ALPHA）；
- **没有**创建 `release/v4.1.0` 分支或 `v4.1.0-rc1` / `v4.1.0` tag（切 tag 属人）；
- **没有**替人类架构师生成签批行；
- **没有**复制 v4.0.0 的「GA (CONDITIONAL PASS)」模式（该模式已于 2026-09-30 撤销）；
- **没有**删除或改写任何历史归档 run 的 manifest（审计清单门 10 条存量红只诊断、
  未擅动；处置形态见 #5794 / #5596 / #5698 的待裁项）。

## 5. 发布判定的原则（不可协商）

> 无实跑证据 ⇒ 不写 PASS。模型意见 ⇒ 仅作 soft gate（AFP §5.1），不作门禁依据。
> 「测不到」≠「通过」；「文件存在」≠「证据成立」。

## 6. 下一步（按可执行性排序）

| 序 | 动作 | 责任方 |
|---|---|---|
| 1 | 让 168h SOAK 独占窗口跑满（起跑时刻由 Z6G4 安静窗口门禁决定），两臂产物齐备 | 执行（**守门启动器已起，等待主机安静窗口**） |
| 2 | 跑完 GA 35 项实跑并按结果清零可消红项 | 执行 |
| 3 | 恢复 owner 2026-11-08 裁决的 23 道「执行点」（或从声称面撤下） | 执行 + 属主裁定 |
| 4 | 建 `release/v4.1.0` 承载分支并落 RC 内容 | 执行 + 属主授权 |
| 5 | 独立 Reviewer 回执（Hub #133） | 属主 / Reviewer |
| 6 | 人类架构师签批 RC→GA | **属主** |
| 7 | 镜像同步（gitea250 / gitcode / gitee / github） | 属主（§3.2 禁止 Agent push） |

## 7. 168h SOAK 的执行实况（2026-11-16，如实登记）

> 本节只陈述**实测到的**内容，不预判结论。

### 7.1 第一次在本机启动的那一轮是「空转绿」，已停

臂 A 的负载由 `scripts/soak/v400_1h_soak.sh` 的 pymysql 驱动施加。本机实测：

- `driver.stdout` = `FATAL: pymysql not installed. pip install pymysql`
- `driver.log` **从未生成**
- 而 `metrics.csv` 已有 210 行「健康」采样：`driver_q=0`、RSS≈8MB、FD=10、panic_count=00

服务端健康地空转，负载为零。若只看 metrics 与退出码，会误以为「在跑」。
该轮已停，产物留在 `/Volumes/workspace/dev/soak-artifacts/20261116-168h-v410`
（**仅预检价值**）。

### 7.2 补上承重断言（PR #5809，合并 `baa3f769c2`）

| 断言 | 实测 |
|---|---|
| 预检缺 pymysql ⇒ 拒跑 | `PRECONDITION FAILED: pymysql is required by the arm-A load driver`，rc=2，**零产物目录**（没有消耗任何窗口） |
| 驱动秒死 ⇒ 立刻判红 | 2 分钟窗口的 run 在第 **16 秒**报 `FAIL-CLOSED: driver died at elapsed=15s`（改前要空转到时长结束；168h 场景即 7 天） |
| 产物必须带负载证据 | 判定为合取：退出码 0 ∧ 产物齐全 ∧ 负载已施加 |

252 上的绿路径冒烟（3 分钟，两臂全开）**两臂 PASS**：臂 A 查询 8439 / 错误 6 /
覆盖 102%；臂 B attempted 1979 / succeeded 1979 / failed 0、五类齐全、harness 判 PASS。

### 7.3 Z6G4 的第一轮 168h 被主机噪声污染，已主动停掉

起跑时刻 02:59:52，而**同一台机器**上 root 的作业在 02:59 开始：

```
/workspace/openclaw/sqlrustgo/target/debug/deps/oltp_test-…   占 29 核
cargo test --workspace --exclude sqlrustgo --no-fail-fast    timeout 17400s ≈ 4.8h
cargo test --all-features --no-fail-fast                      timeout 16800s ≈ 4.7h
```

80 核的机器 load average 冲到 **89.48**，本 run 的服务端只拿到 8.5% CPU。

| 指标 | 受污染的 168h run | 同一脚本的 3 分钟冒烟（root 作业起之前） |
|---|---|---|
| 成功查询耗时 | 9.4 s | 正常量级 |
| 超时查询耗时 | 15 s | — |
| 错误率 | 9.6%（89 / 1298） | 0.07%（6 / 8445） |
| 吞吐 | 4.7 q/s | 47 q/s |

**那是主机争用，不是被测系统的缺陷**；但一份被争用写出来的 SOAK 报告，
证明的是那几分钟的争用。宁可晚起跑，不可带噪起跑。该轮已停并原样留档：
`~/soak-artifacts/20261116-168h-v410-CONTAMINATED/CONTAMINATED.md`。

### 7.4 守门启动器（PR #5810，合并 `902df14a6`）

`scripts/soak/run_v410_168h_soak_on_z6g4.sh` 两道门禁：

1. **安静窗口（起跑前）**：无外来重负载（无他人 `cargo build/test`、`rustc`、
   `oltp_test`、`sysbench`，无他人进程吃 ≥50% CPU；forge 自己的 postgres/gitea
   不算）+ load 均值 ≤ 40，连续满足 10 次（每次 60 s ⇒ 10 分钟）。
2. **吞吐复验（起跑后 180 s）**：驱动真实 QPS ≥ 5，否则停掉这一轮、退回等待。

两个承重的实现细节：**只把「别人 owner」的作业算作争用**（启动器自己起的臂 B
就是 `cargo test`，若不按 owner 区分会把自己起的 run 判成争用而永远起不了跑）；
**每轮 attempt 独立产物目录**（复验读的是 `driver.log` 的 Q 行数，沿用上一轮
目录会把旧账算进来，复验门禁即废）。

### 7.5 当前状态（截至 2026-11-16T19:08Z；后续进展见 §7.7–§7.12）

- 冻结提交 `3264b5dafd36cca7c52f03154d86d3945eb4f7b9`（252 上 dirty=0；2026-11-16T19:15:58Z 起的新版启动器用的是这一版）
- release 二进制 sha256 `de7c4e09445d9697dedcd612e0ffabe0e3a1b63df242e2e8c7b732173cd1512f`
  —— `crates/` 在 `baa3f769c2`→`902df14a6` 间**未变**，故已构建二进制确属该冻结提交
- pymysql 装在 `~/venv-srg`（venv；该机 PEP 668 管控，`pip --user` 不可用）
- 启动器 2026-11-16T19:08:18Z 实测判 **BUSY**（load=41 + root 的 cargo 作业）⇒ 等待安静窗口，**尚未起跑**

起跑时刻由门禁决定，不由人决定；`~/soak-artifacts/20261116-168h-v410/STARTED.json`
会写下起跑时的冻结提交、二进制指纹、起止时间与复验 QPS，作为规范记录。

### 7.6 驱动的证据文件此前是假的（PR #5812，合并 `3264b5dafd`）

`scripts/soak/v400_soak_driver.py` 写证据文件的方式是：Q 行**每 500 次**才
flush 一次，E 行**从不 flush** —— 落盘实际靠 8 KB 缓冲区隐式完成。两个后果：

1. **证据不可读**：慢一点的 run 几分钟内一个字节都落不下去。
2. **错误数不真**：E 行不 flush ⇒ 报告里的错误计数是「缓冲区最后一次冲出去
   之前」的数，不是总数。

第 1 条还直接导致 §7.2 那道「驱动活着但零查询 ⇒ 判红」的断言**误伤**：
本机一个 2 分钟窗口的 run 里 `driver.log` 全空（驱动其实在跑，账在缓冲区
里），断言在第 61 秒判红。**判据本身没错，但它读到的是缓冲区的账。**

修法：Q 行与 E 行**每行都 flush**。复测（2 分钟臂 A，venv + pymysql）：

```
rc=0  耗时=128s
ts,elapsed_s,rss_mb,fd_count,cpu_pct,driver_q,driver_e,panic_count,error_high,host_load1,host_busy
2026-11-17T03:14:57,121,43.9,11,3.0,7793,3,00,0,12.92,1
PASS — 7793 queries via 16 threads in 2min, 3 errors, 0 high-CPU episodes.
```

同时把**主机争用**纳入证据：臂 A 的 metrics.csv 新增 `host_load1` /
`host_busy` 两列（逐采样记录主机负载均值与「是否有进程吃掉 ≥50% 核」），
FINAL_REPORT 新增「主机争用」行与偏差说明。SOAK 的数字只有放在主机状态
旁边才可读 —— 机器被别人压着时，「慢」说的是机器，不是服务端。

启动器的「外来重负载」判据也相应放宽为**只算正在干活的**：停在 0 % CPU
的 cargo build（在等锁/等网络）此刻并没有抢机器，把它算成争用会让 SOAK
永远起不了跑 —— 那是把「有人在旁边待着」误当成「有人在抢」。它一开始
编译，rustc 子进程会被立即命中；安静窗口要求连续 10 次满足，间歇性抢
机器照样把计数清零。


### 7.7 启动器的仪器错了三处 —— 门禁从「保护」变成了「死锁」

安静窗口这道门禁的「外来 CPU」读数，此前是 `ps -eo pcpu` 求和。三个错误叠在
一起，每一个单独都足以让这道门禁失效：

1. **口径错**：`ps` 的 pcpu 是**进程生命周期均值**，不是当前占用 —— 四小时前
   启动、现在空闲的进程，仍按它一生的均值计费。
2. **漏算**：**已经退出的进程完全不被计入** —— 而别人的编译负载恰恰是
   「spawn 一个 rustc、烧 30 秒、退出」，样本之间这些 CPU 一个都不算。
3. **单位错**（换增量口径后踩的）：系统侧算成「整机百分比」（0–100）、自有侧
   算成「核百分比」，两者相减 ⇒ 外来 CPU **恒等于 0** ⇒ `[ "0" -gt "2400" ]`
   恒为假 ⇒ `FOREIGN_CPU_MAX` **永不触发**。

同一台 80 核机器、同一组别的作业，两种口径实测：

| 口径 | 读数 | 折算 |
|---|---|---|
| `ps` 生命周期均值 | 7985 % | = 80 核（看起来满载） |
| 增量（修正单位后） | 674 – 1784 % | = 6.7 – 17.8 核（实际占用） |

即 `ps` 口径虚高约 **4–11 倍**。用它卡门禁，等于要求一台「80 核全空闲」的
机器；而 252 上 root 的 CI 常驻（`bustubx-unpause-ci.sh` 连续跑 4 h 20 m，不断
拉起 cargo build/check/test/clippy）⇒ 旧口径下安静窗口**永远不会满足** ——
**等下去不是保护，是死锁**。

修正后的口径：**系统级 busy（/proc/stat 差分）− 自有进程 busy（/proc/<pid>/stat
差分）**，单位统一为「核百分比」（100 % = 1 核），与 2400 % 的门禁同口径，且
**不漏短命进程**。标定不失效：污染轮里 root 的 `oltp_test` 独占 29 核 =
2900 % > 2400 % ⇒ 仍被拒；干净轮与争用探针（61 q/s、0.12 % 错误）仍被收。

第四个问题在采样解析：`sample_cpu` 的首轮输出是 `warmup|0|0`（还没有基线），
而解析按 `cut -d'|' -f1` 取值 ⇒ 拿到字符串 `warmup` ⇒ bash 对
`[ "warmup" -gt 2400 ]` 返回非零 ⇒ `&& QUIET=0` **不执行** ⇒ QUIET 保持 1
⇒ **一个没读出来的采样被当成一次合格的安静采样**。现改为三态：warmup /
读数 / **读失败**，读失败与 warmup 一律判「非安静」并落 WARNING（把未知当成
事实，是这类门禁出错的根）。

复验同时加严（PR #5819）：`MIN_DRIVER_QPS` 5 → **10**（旧值贴着实测的受污染轮
4.7 q/s，只差 6 % 余量，等于没有余量；10 距污染轮 2.1 倍、距健康轮 4.7 倍）；
新增**错误率 ≤ 1 %**（与臂 A / 臂 B 同口径）；复验只看驱动启动 30 s 后的
**稳态区间**（热身段的错误不代表稳态）。另补一条兜底：等满 `QUIET_WAIT_MAX`
（默认 2 h）仍未凑齐安静窗口时，只要 load1 ≤ 40（机器**没有过载**）就允许
试跑一次，**收不收由复验决定** —— 复验量的是我们自己的真实吞吐与错误率，是
**结果指标**，比「机器看起来安静」这个代理指标可靠。

### 7.8 臂 A 此前没有错误率判据（PR #5818）

臂 A 的判定此前只有三类：负载是否真的施加上去、服务是否崩溃、查询数是否够。
**没有错误率判据** —— 驱动把 9.6 % 的查询打成错误，臂 A 照样写 PASS。那样
一份 PASS 证明不了任何事：它分不出「被测系统健康」与「被测系统已经坏了但
没崩」。臂 B 一直有这道判据（`v312_mixed_soak` 的 `GA failure rate exceeds
1 % threshold`），**两臂口径不一致本身就是缺陷**。

阈值取 1 %（与臂 B 同值），落在 252 的三个实测点之间：干净轮 6/8445 =
0.07 %、争用探针 9/7379 = 0.12 %（均远低于上限）、受污染轮 89/1298 =
9.60 %（超上限 9.6 倍，改前会被判 PASS）。判据从**原始证据**（driver.log 的
E 行）重算，不读 summary.json —— 报告里的数字与判据用的数字必须同源。退出码
**4** 是新增的，用来区分「跑挂了」（1/2/3）与「跑完了但已经退化」。

252 上两条路径都实跑过：

```
验证 #1（默认 1%）  ⇒ PASS — 2857 queries via 16 threads in 2min, 00 errors (0.000%, max 1%)   rc=0
验证 #2（上限 0%）  ⇒ FAIL — driver error rate 1.636% exceeds 0% (15 errors / 917 attempts)  rc=4
```

### 7.9 两条被 `#[ignore]` 掩盖的真实缺陷（PR #5823）

反-ignore 门禁（G19）查出台账漂移：树里 **137** 个真实 `#[ignore]`，台账只
登记 **136** ⇒ 有一条**从未登记**。顺着这条线挖出两个真实产品缺陷：

**（一）页式存储下 `CREATE INDEX` 让已有的行消失**（看板行 IDX-2）：

```
SELECT id FROM d WHERE v = 20;   => 1 行
CREATE INDEX idx_d_v ON d(v);
SELECT id FROM d WHERE v = 20;   => 0 行    ← 建索引后丢行
```

根因不是「行号编码不对」，而是**行号语义两侧不一致**：写侧把行号当成
「`tables.rows` 里的下标」，而页式后端的行根本不在 `tables.rows` 里（它们在
页中）⇒ 建出的是空树；读侧又按同一套（错的）语义解析行号 ⇒ 查询返回 0 行。
修法：两侧用**同一套语义** —— 行号 = 表行序里的位置（页式后端即
`insert_via_bufferpool` 记下的页序 = `scan_via_bufferpool` 遍历的顺序）。
此前属主口径是「要不要把 row-id 改成 (page, offset)」—— 本次证明那不是唯一解：
**语义统一即可**，回归面小得多。

**（二）MySQL 长度编码整数三字节分支被 ×256**：`decode_lenenc_int` 的 0xfd
分支写成 `u32::from_le_bytes([0, b0, b1, b2])` —— 0 落在**最低**字节 ⇒ 每个
字节移了一位 ⇒ 解出的长度是真实值的 256 倍（0xfc/0xfe 分支无此错位）。
该函数在**生产路径**上（解析 VARCHAR/BLOB 列值长度）⇒ 64 KiB–16 MiB 的
文本/blob 值会被解成 256 倍 ⇒ 长度校验失败 ⇒ 该行解码崩掉。

修后实测：`cargo test -p sqlrustgo-storage --lib` ⇒ **916 passed / 0 failed**；
`cargo test -p sqlrustgo-mysql-server --lib` ⇒ **287 passed / 0 failed / 0 ignored**；
`cargo test --test idx1_session_db_index_test` ⇒ **5 passed**（IDX-2 那条不再
被忽略）；P16 ⇒ `48 gate tests, 0 NEW #[ignore]` ⇒ **PASS**；G19 ⇒ tree 135 =
registry 135 ⇒ **PASS**。台账同步：`total_allowed` 136 → 135，顺带把台账里
**本来就陈旧**的描述字段 `known_deficiencies.active` 由 69 改为实算值 81。

### 7.10 TPC-H 哈希基线重锚（PR #5822）

`tests/tpch_hashes_v380.json` 的 `b8854271…` 是 2026-06-28 在
`545814c3d1 (v3.9.0)` 上采的。**本仓历史里没有任何生成器能产出那份数据集**：

| 夹具 | customer / lineitem 行数 | 22 条聚合哈希 |
|---|---|---|
| 旧基线（v3.9.0） | — | `b8854271…` |
| `generate_tpch_sf.py`（2026-07-18 才加入，SF=0.001 / seed 42） | 150 / 6018 | `7439ce80…` |
| `generate_sf001_fixture.py`（采集时在树里、后被删；其「SF=0.001」行数只有标准值 1/10） | 15 / 501 | `d7ec506b…` |

即自 v3.9.0 起，**引擎的 TPC-H 输出与夹具都变了** ⇒ 旧基线不可复现，G1 只能
永远 INCOMPLETE。新基线按 issue #5688 的口径**逐条用独立 SQLite oracle 复算**
（明确不用 `--generate-baseline` 的引擎自捕获 —— 引擎自证正是要消灭的形态）：

```
statements=22   oracle_set_equal=21   unresolved=[]
唯一不等的一条 Q17：row 0 col 0: engine="" oracle="<NUL>NULL"
  （wire 协议把 SQL NULL 渲染成空串；dump 自己单列为 null_representation_only，不计入 unresolved）
```

同时给基线补上**数据集身份**校验：基线记录 generator + generator_sha256 +
seed + row_counts，夹具的 manifest 带同样字段；身份不符 ⇒ 披露
`TPCH-NOT-RUN … reason=fixture_identity_mismatch`，**不得**误报漂移。此前只比
scale factor，而同一个 sf 可以由不同的（生成器, 种子）产出 —— 那不是漂移，
是「比的不是同一个东西」。

实测（两个用例都跑过）：身份匹配 ⇒ `G1 PASS: TPC-H 22/22 baseline hash matches
(7439ce80…)`（50 passed / 0 failed）；身份不匹配 ⇒ `TPCH-NOT-RUN … reason=
fixture_identity_mismatch`（**没有假漂移**）。门禁侧：`check_g1_tpch_baseline.sh`
⇒ **G1 PASS**；同一份 hermetic SF=0.001 夹具（已部署到 252 的
`~/sqlrustgo-tpch/data`，8 个 .tbl + manifest）使 `check_p22_time_travel.sh`
与 `check_g16_compatibility.sh` 由 INCOMPLETE 转 **PASS**（两者均报
`TPCH-SUBCHECK: PASS`）。旧值留档在基线文件的 `previous` 字段，不删。

### 7.11 35 项 GA 门禁实跑的逐项归因

实跑 `20261116-ga-gates-003`（32/35：PASS 23 / FAIL 6 / INCOMPLETE 3）的红项
根因与处置：

| seq | 门禁 | 根因 | 处置 |
|---|---|---|---|
| 1 | `check_rc_ga_gate` | D1 的 A2 测试 + A5 覆盖率**测不到** `sqlrustgo-storage` | PR #5816（VtuGuard 事务状态被 trait 默认值静默丢弃）⇒ storage 单测 916 passed / 0 failed |
| 16 | `check_anti_fabrication` | V312-24 test-runner 挂了 `vtu_guard::tests::test_vtu_guard_tx_state_via_set_current_tx_id` | 同上 |
| 20 | `check_full_gate_verification` | 承接 seq1（D1–D5 维度） | 同上 |
| 24 | `check_gate_test_integrity` | P16：多 1 个 `#[ignore]`（IDX-2 那条） | PR #5823 修缺陷 + 撤销 ignore ⇒ P16 实测 PASS |
| 26 | `check_coverage_v312` | 同 seq1 的覆盖率根因（storage 跑不起来 ⇒ exit 101） | 同 PR #5816 |
| 31 | `check_anti_ignore_gate` | 台账漂移：树 137 vs 台账 136 | PR #5823 ⇒ G19 实测 PASS |
| 11/13/14 | P22 / G1 / G16 | **同一根因**：TPC-H 夹具缺失 | 部署 hermetic SF=0.001 夹具 + PR #5822 ⇒ 三项实测 PASS |

**003 轮本身已作废**：其工作树在实跑中途被我更新过（PR #5818–#5822）⇒
前后两段门禁跑的不是同一棵树，结果**不同源**，不能作为 GA 证据。权威实跑必须
在**最终提交**上重跑。

### 7.12 当前状态（2026-11-17）

- 252 上的守门启动器**已按新版口径实跑采样**（20:46 起连续 QUIET：load 21–31、
  外来 CPU 1176–1533 %，均低于 2400 % 的上限；绑定门禁是 load1 ≤ 40）。
- 20:51 起**主动暂停**并留档（`launcher.log` 的 `PAUSED` 行）：冻结提交
  `6129aa064d` 缺少 IDX-2 修复（crates/ 变更 ⇒ 二进制变更）⇒ **SOAK 必须与
  GA 候选同源**，故待最终提交确定后用同一策略重启。
- 权威实跑（run 005）将在最终提交上启动；35/35 尚未取得。
- 仍需属主/人类完成的事项不变：人类架构师签批（RC→GA）、承载分支
  `release/v4.1.0`、Hub #133 独立 Reviewer 回执、168 h SOAK 跑满；另两个
  **已过期**的 ADR-008 例外（2026-09-01 / 2026-10-31）需属主重新签发。
