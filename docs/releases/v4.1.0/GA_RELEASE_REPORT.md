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

### 7.5 当前状态

- 冻结提交 `902df14a6f2e6c2f7df505c4dce1437e3455faaf`（252 上 dirty=0）
- release 二进制 sha256 `de7c4e09445d9697dedcd612e0ffabe0e3a1b63df242e2e8c7b732173cd1512f`
  —— `crates/` 在 `baa3f769c2`→`902df14a6` 间**未变**，故已构建二进制确属该冻结提交
- pymysql 装在 `~/venv-srg`（venv；该机 PEP 668 管控，`pip --user` 不可用）
- 启动器 2026-11-16T19:08:18Z 实测判 **BUSY**（load=41 + root 的 cargo 作业）⇒ 等待安静窗口，**尚未起跑**

起跑时刻由门禁决定，不由人决定；`~/soak-artifacts/20261116-168h-v410/STARTED.json`
会写下起跑时的冻结提交、二进制指纹、起止时间与复验 QPS，作为规范记录。
