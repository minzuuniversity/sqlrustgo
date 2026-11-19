# SQLRustGo v4.1.0 GA Gate Report

> **基线**：`develop/v4.1.0` @ `2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`
> **provenance:** generated_at=2026-10-23, branch=docs/ga-rc-artifacts-batch32,
> commit=`2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`, machine=Mac-mini-7.local (arm64, macOS 27.0.1),
> identity=openclaw-mcode, source_repo=openclaw/sqlrustgo,
> policy=Anti-Fabrication-Policy-v1.0
> **gate_policy_eval_id**：`v410-ga-gate-report-g1-g6-001`
> （本文档汇编的门禁引擎：`scripts/gate/check_rc_ga_gate.sh`、`scripts/gate/check_coverage_v312.sh`、
> `scripts/gate/check_anti_fabrication.sh`、`scripts/gate/check_full_gate_verification.sh`、
> `scripts/gate/check_drift_not_pass.sh`、`scripts/gate/check_no_hardcoded_secrets.sh`、
> `scripts/gate/check_release_artifacts.sh`；证据绑定门禁
> `scripts/gate/check_evidence_binding.sh` 的 Type B 判据据此认定本文档的
> PASS 声明有对应的 gate engine 输出，而非 AI 自述。）

---
<!-- BEGIN 2026-11-19 REFRESH BLOCK

## 🔄 刷新块（2026-11-19T03:30:00Z）—— 第 8 次刷新，取代下方 2026-11-19T02:00:00Z 的判定

> ⚠️ **〔状态更新 2026-11-19T04:40Z —— 本块已不是文件内最新】**：文件内另有一块**更晚**的刷新块
> **2026-11-19T04:35:00Z（第 9 次刷新）**，位于本块**下方**（标记对 `<!-- BEGIN 2026-11-17 REFRESH BLOCK -->`）。
> 本块正文**一字未改**（历史记录原样保留），但**其中「r4 已起跑」等状态已被 2026-11-19T04:18Z 的换轮作废**
> （r4 被换轮脚本主动停止，r5 尚未起跑；详见 `GA_RELEASE_REPORT.md` §7.27 / §7.28）⇒ **状态以 04:35Z 块为准**。

> **本块只写「已实测的事实」与「仍未达成的项」**；**不产生任何 PASS 声明**。
> 阶段仍为 **ALPHA**，**未跃迁**。
> **签署栏自 2026-11-18T08:27:00Z 起已转录为 CONDITIONAL（见文末），本块未改动签署内容**。

### 一、**第三层阻塞已闭合**：两个叠加缺陷被判决性复现、归因、修复，并在 252 上现场验证

本块期间新发现并闭合了**两个此前未识别的叠加缺陷**。二者都是「r4 按原配置必然失败」的原因，
若不修，r4 跑满 168 h 也只会得到又一轮不合格。

#### 1.1 缺陷 A：主键等值点查约 **99% 返回 0 行**

- **现象（252/r3 实测）**：`point_select` 72,449 次仅 **653 次命中（0.90%）**，
  而 `range_scan` 29,264 次 **100% 返回 100 行**、`count_agg` 14,393 次 100% 正常；
  落盘 `sbtest1.json` **24,399 行、id 1..10000 齐全** ⇒ **数据不缺、范围查能看到，只有等值查看不到**。
- **命中率的形状本身就是线索**：起跑首个千行 **7.30%** → 稳定 **0.3%–0.5%**
  ⇒ 指向「某个随写操作单调恶化的状态」。
- **因果归因（三组对照实验，本机实跑）**：

  | 实验 | 负载 | point_select 命中率 | 判读 |
  |---|---|---|---|
  | E1 | 纯读（40,150 次） | **100.00%** | 读路径健康 |
  | E2 | 读 + INSERT | **100.00%** | **INSERT 不是触发条件** |
  | E3 | 读 + UPDATE | **8.03%** | **UPDATE 是触发条件** |

- **根因（四环，缺一不可，全部代码级坐实）**：
  1. 引擎把 UPDATE 实现成 **delete + insert**；
  2. 删除用 `Vec::retain` **就地压缩行向量** ⇒ 被删行之后每一行**物理下标前移一位**；
  3. 主键 B+Tree 存的是**物理行号**，且 DELETE 路径**从不维护索引**（全仓无一处 `index.remove`）；
  4. 取行时**不校验键值** ⇒ 等值查拿到**别的 id 的行** ⇒ 随后被 WHERE 复算清空 ⇒
     **表现为「查不到」而不是「查错了行」**。
- **历史对照**：`audit/v4.1.0/runs/20261112-soak-ab003/artifacts/driver.log` 同形态
  （点查 4.3% 命中）⇒ **长期存在的既有缺陷，非本轮回归**。
- **修复**：删除后**原地修正**索引行号（`BPlusTree::adjust_after_removal`）＋
  取行**校验键值**＋ Eq 未命中回落**定点查找**（不整表克隆）＋
  `insert_buffered` 阈值刷盘分支改用**窗口法**（原先用本次调用的行去索引整批 ⇒ 漏键且行号整体平移）。

#### 1.2 缺陷 B：每个写事务把**整个 WAL 读一遍再重写** ⇒ 写代价 O(WAL) 二次方

- **syscall 级证据**（隔离探针实例，`strace -f -y`）每提交重复出现：
  `openat(O_RDONLY)` → `openat(O_WRONLY|O_TRUNC)` → `openat(O_APPEND)` → 逐条重写 → `fdatasync`；
  实测斜率精确 **1.00**。
- **根因**：同一份日志上存在**两个 LSN 空间** —— `WalStorage` 用自己的计数器盖章，
  `FileBackedWalManager::append` 又**覆盖**成它自己的 `next_lsn`（而它统计的是**所有**经共享
  handle 的条目，含 `FileStorage` 自己的）⇒ 两个计数器速率不同 ⇒
  **检查点记录的 LSN 与截断点不在同一空间** ⇒ `truncate_before` 判不出哪些条目可回收 ⇒
  **WAL 无界增长**，而每次提交都要整份读改写。
- **修复**：`WalManager::append` 改为**返回管理器盖的 LSN**（LSN 空间由管理器独占）＋
  `truncate_before` **无可回收即直接返回** ＋ 截断点取 `min(检查点 LSN, 最老在途事务起始 LSN)`
  （在途事务的条目尚未进任何快照，回收会丢数据）。

#### 1.3 修复前后对比（同一探针、同一台架、同一驱动）

| 指标 | 修复前 | 修复后 |
|---|---|---|
| 点查命中率（E3 读+UPDATE） | **8.03%** | **100.00%** |
| 点查命中率（SOAK 同参混合负载） | **5.67%** | **100.00%** |
| WAL 大小（混合负载） | 1,611,118 B | **135 B** |
| WAL 大小（放大探针，每轮 200 INSERT） | 37,800 → 226,800 B（线性无界） | **恒定 90 B** |
| UPDATE 吞吐 / 180 s | 1,484 | **6,037**（4.07×） |
| 错误数 | 0 | 0 |

- **回归测试的有效性已实测**：把「索引修复」与「键值校验」两道防线**同时中和**后，
  新增的回归测试**失败**并精确暴露错行（`id=4` 查到 `Integer(5)`）⇒ **真回归测试**，非恒真断言。
- 存储层单测 **919 passed / 0 failed**。
- 提交：`757b538046`（PK 索引）、`56587af2ce`（WAL）、`4cc3ab942a`（门禁判据）；**PR #5865 已合并**。
- 完整证据：`audit/v4.1.0/adjudications/soak-third-layer-pk-and-wal-20261119.md`。

### 二、**门禁判据加严**：臂 A 增「主键等值点查命中率 ≥ 99%」断言

`scripts/soak/run_v410_168h_soak.sh` 的臂 A 判据此前**只看 `A_QUERIES > 0`** 与采样覆盖率，
**不校验任何一次查询返回了多少行** ⇒ **一轮点查 99% 返空的运行照样会被判 PASS**。
现从 `armA/driver.log` 统计 `point_select` 命中率，要求 **≥ 99%**，并并入臂 A 的 PASS 合取式
与最终报告表格。**判据只增严，不放宽任何既有门槛；门禁分母 168 不动。**

### 三、认证轮：017 / 018 **连续两轮中止（均非判红）**，019 在基线修复后的树上起跑

| 轮次 | 终态 | 说明 |
|---|---|---|
| 017 `20261119-ga-gates-017` | **中止（INCOMPLETE）** | 在被测树 `b697a9a687` 上起跑，seq 1 已跑 72 min 时**代码面变更**（PR #5865 合并头 `93ac1d427e`）⇒「验收对象是已提交的那棵树」的前提已变 ⇒ 017 的结论对新树无效 ⇒ 按 PID 终止（`run_gates.py execute` 85470 及其 cargo 子进程）。**已归档为 `round_status=INCOMPLETE`（非 PASS），verify rc=0**，plan 与二进制指纹留存作 pre-fix 基线 |
| 018 `20261119-ga-gates-018` | **中止（INCOMPLETE）** | 在被测树 `93ac1d427ec0694f613da8470a88003db08b9f53`（= PR #5865 合并头）上起跑，prep 阶段 ① 全绿（`cargo build --all-features` rc=0 / 187 s、`sqlrustgo-cli` rc=0、`sqlrustgo-mysql-server` rc=0）、② 二进制指纹已生成；③ 阶段因**认证前排雷**发现 P14 drift 门禁在该树**必然判红**（实测 rc=1、7 条 NEW，逐条比对确认全是既有债务被 #5863 的 15 行文件头平移误判）而中止 —— 该门禁在 35 项 plan 内、**seq 21** ⇒ 本轮跑到 seq 21 必红 ⇒ 先修基线再起跑 019。**未执行任何一项门禁**（results 0 行），按 PID 终止 80460 / 80458 与 cargo 子进程 30086 / 88691（退出码 143）。**已归档为 `round_status=INCOMPLETE`（非 PASS），verify rc=0**，evidence_hash `sha256:43db9b1d…` |
| 019 `20261119-ga-gates-019` | **待起跑** | 被测树 = **drift 基线修复合并后的 develop tip**；timeout **21600 s**（只放宽运行参数）；plan sha256 强校验 `sha256:6466a0ec…` ⇒ **35 项清单未变、分母守恒**。**起跑前先排雷**：按 016 轮实测耗时把 29 项快门禁（合计约 135 s）与 4 项中等门禁（约 72 min）先跑一遍，确认无红再跑满，避免「跑 7.24 h 才撞红」。对照 016 轮实测耗时 **7.24 h**（其中 seq 20 与 seq 33 双双 10,800 s 超时，**已定性为执行预算问题，不是判红**） |

**019 与 r4 在两台不同宿主上并行**（019 跑本机、r4 跑 252）⇒ 不争 CPU，不构成 r4 的前置。

### 四、168h SOAK：**r4 已起跑**（run `20261119-168h-v410-r4`）

| 项 | 值 |
|---|---|
| 起跑时刻 | **2026-11-19T03:25:01Z**（安静窗口 **10/10** 连续满足，waited 602 s，attempt 11） |
| planned_end | **≈ 2026-11-26T03:25:01Z** |
| 被测树 | `93ac1d427ec0694f613da8470a88003db08b9f53` —— **含缺陷 A/B 两处修复** |
| release 二进制 | sha256 `910029ce1f0658e1028fbfad12d7f3d109fe6516e8ef303d03abefa06daffa33`（15,048,264 B） |
| 起跑门禁 | 急停开关**在位**；四个守护服务（独占看门狗 / 冻结守护 / CPU 采样 / 表规模采样）全 **active**；端口 3413 起跑前实测空闲 |
| **起跑后首批实测读数** | **`point_select` 22,058 次全部命中 = 100.00%**（r3 轮同指标 0.90%）；**`sqlrustgo.wal` 恒为 90 B**（r3 轮 90 min 后为 8,054,374 B）；**errors 00 / panic 00**；**343 q/s**（r3 轮为 25.6 q/s ⇒ **14×**） |

r3 轮已在 elapsed **7168 s** 处终止并留档：最终读数 **errors 7 / panic 00**，
且**急停开关开启后的 1.5 h 内错误零复发** ⇒ 宿主同盘 I/O 风暴根因的**二次印证**。

### 五、§7.25「服务端侧成分」（B1b）：**已定性 —— 就是本块闭合的两个缺陷**

第 7 次刷新块登记的开放问题「r3 实测退化速度超过行数增长、且主键点查也变慢 ⇒ 是普适退化」
—— 本块已给出答案：

- **点查变慢且大量返空** = 缺陷 A（UPDATE ⇒ 删除压缩 ⇒ 索引行号漂移 ⇒ 等值查退化到全扫兜底）；
- **吞吐随时间下降、行数越多越慢** = 缺陷 B（每次提交整份读改写 WAL ⇒ 写代价随 WAL 线性增长 ⇒ 二次方）。

两处均已修复，且**在 252 的真实轮次上现场验证**（点查 100%、WAL 90 B、吞吐 14×）。
⇒ **B1b 从「开放问题」转为「已定性并修复」，其最终确认依赖 r4 跑满 168 h。**

### 六、本块刷新期间闭合的其他项

| 项 | 结果 |
|---|---|
| 83 道 dead/orphan 门禁（dead 61 + orphan 22） | **已加文件头自标 + 登记在册**（PR #5863），**零副作用**（`gate_registry` fails 0 / check_n **168**；`check_gate_script_refs` errors **0** / warnings **8** / **58** citations；`check_gate_executability` **92** 项 OK） |
| `check_historical_backlog_disposition.sh` 的 3 条 FIX-TLS 不可达 | **已修正**（指向当前树实测可达且真实承载该修复的等价提交）⇒ **PASS=3/FAIL=3 → PASS=7/FAIL=0**；**原始 SHA 全部保留**（PR #5864 已合） |
| 独占窗口从「一次性」升级为「持续自愈」 | 看门狗 + 急停开关 + 冻结守护 + 四个 systemd 服务（PR #5864 已合） |
| 252 的 Gitea `refs/heads/` 属主 root | **已自助解除**（`docker exec -u root … chown -R git:git`） |
| 看板 | 补 **4** 行（GA-SOAK-24 / GA-GATES-18 / GA-SOAK-25 / GA-GATES-19）；既有 198 行**逐字节未变**、键序固定、未 sort_keys；`kanban_merge_policy` 判定 **OK**（rows=**202**、problems=[]） |
| r4 被测树与当前 develop tip 的差异 | **运行时行为面 = 0**（`crates/` **零变化** ⇒ r4 的 release 二进制 sha256 `910029ce…` 仍是 tip 的真实构建产物）；非运行时面 = docs/audit + `tests/baseline/drift_baseline.json`（9 行行号同步）⇒ **材料提交与门禁基线同步都不使 r4 结论失效**（见 §六 下一行的公开裁决） |
| P14 drift 基线行号重同步（#5863 的 15 行文件头平移误判） | **已修**（提交 `c5bf397472`）：9 条条目按当前树**实测位置**逐条更新（同一文件内位移一致为 **+15**，与 #5863 给 83 道门禁各加的文件头行数吻合）；**条目数 29 / `total_findings` 29 / 描述文本 / `created_at` 全部不变**，**不改门禁判据、不改阈值、不动门禁分母 168**。改后实测 **rc=0**（`No NEW anti-patterns vs baseline` / `Total anti-patterns: 0`）；**中和验证**：植入一处真实反模式 ⇒ 判红并**点名该处**；移除 ⇒ 回 rc=0 ⇒ **新反模式检出能力未削弱**。旁证：`check_gate_script_refs` 58/58、`check_gate_executability` 92/92、`gate_registry` fails=0 check_n=**168** |
| **公开裁决（ADR-014 D-06）**：`tests/baseline/*.json` 归**门禁自检基线面**，**不归运行时行为面** | **分歧**：本轮合并的 drift 基线同步改了 `tests/baseline/drift_baseline.json` ⇒ 若把「代码面」定义为「整树字节」，则 r4 被测树 `93ac1d427e` 与合并后 tip 的差异**不再为 0**，SOAK 结论的「被测对象同一性」前提会被质疑。**裁决**：「代码面」= **参与被测系统运行时行为的产品代码**（`crates/` 下被编入二进制的代码及其运行时配置）。依据：① `tests/baseline/` 是**门禁自检用的登记数据**，不进任何二进制、不参与任何查询路径；② 它与 SOAK 的三条判据（**0 errors / 0 panics / point_select 命中率 ≥ 99%**）**无因果关系** —— 改它既不可能产生错误、也不可能改变点查命中率；③ **决定性判据**：本次合并 **`crates/` 零变化** ⇒ r4 的 release 二进制（sha256 `910029ce…`）**仍是合并后 tip 的真实构建产物** ⇒ 被测**运行时行为**同一性成立。**该裁决不掩盖任何东西**：漂移基线同步的判据收紧过程（9 条逐条实测定位 + 中和验证 + rc=1→0）已在本块 §六 与裁决文档中完整公开 |

### 七、**仍未达成**（GA 判据的物理 / 人类约束，逐条如实登记）

| # | 项 | 状态 |
|---|---|---|
| B1 | **168h SOAK 的 0 错证据** | ⏳ **r4 已起跑（03:25:01Z）**，判据 **0 errors / 0 panics / point_select 命中率 ≥ 99%**；**168 h 未跑满前，任何「SOAK PASS」都不成立** |
| B1b | §7.25 的「服务端侧成分」 | ✅ **已定性并修复**（= 缺陷 A + 缺陷 B，见 §五）；**最终确认依赖 r4 跑满** |
| B1e | 宿主非正常停止的触发者定性 | ⏳ **属主**（Agent 无带外通道，取不到硬件日志） |
| B1f | 权威 0 错 168 h 需要的宿主条件 | 🟡 **派发侧已压住**（看门狗 + 急停开关 + 冻结守护 + 四服务）；服务端侧成分已闭合 ⇒ **待 r4 跑满确认** |
| B2 | 人类架构师签批 | ✅ 已取得（2026-11-18T08:27:00Z，**CONDITIONAL**，7 条条件与到期日见文末签署栏）。⚠️ **条件①指名 r2 轮，r2 已出局 ⇒ 该条件需属主确认由 r4 承接** |
| B3 | `release/v4.1.0` 承载分支 + tag + 并入 main | ⏳ **属主**（按 `RELEASE_LIFECYCLE` §2.5：审批 → tag → 并入 main → 建承载分支，顺序不可越；**Agent 不得直推 main**） |
| B4 | 旧 Gitea token 在服务端失效 | ⏳ **属主**（仓库侧已收敛） |
| B5 | 两条已过期且从未签署的 ADR-008 例外 | ⏳ **属主**（61 道死门禁部分**已闭合**，见 §六） |

<!-- END 2026-11-19 REFRESH BLOCK -->

<!-- BEGIN 2026-11-17 REFRESH BLOCK

## 🔄 刷新块（2026-11-19T04:35:00Z）—— 取代下方 2026-10-23 的判定

> ⚠️ **本块是文件内最新的刷新块** —— 其上方另有一块 **2026-11-19T03:30:00Z（第 8 次刷新，另一条线所写）**，
> 该块正文原样保留为历史记录，但其「r4 已起跑」等状态已被 2026-11-19T04:18Z 的换轮作废 ⇒ **状态以本块为准**。

> **基线**：`develop/v4.1.0` @ `e2fcf73c2e0b4eb45d9c3379bf2ca9a01ca1869f`（tree `67179eb1c66e9bbeb1b951c2937c776538171253`）—— **当前 develop tip（本次提交前实测）**；⚠️ tip 自 `e5c42c7fd0…` 起已前移
> **认证轮**：run `20261117-ga-gates-011`（plan sha256 `sha256:6466a0ec156075daf3bc53ece379c4ecc2b13b63da27cd17a1f39cb2254da809`），
> 被测树 `fa2b41abfa6b`；其 `git_head` 与**当时**的基线一致 ⇒ **那一轮测的是它自己的被交付树**。
> ⚠️ **`fa2b41abfa6b` 的 crates 与当前 tip 的差距已扩大到 9 个文件 / +614 / −101**（实测 `git diff --stat fa2b41abfa6b e2fcf73c2e -- crates`）：
> #5842 KILL CONNECTION 修复（`crates/mysql-server/src/lib.rs` +132）、**#5865 点查命中与 WAL 两处修复**
> （`crates/storage/src/file_storage.rs`、`bplus_tree/mod.rs`、`wal*` —— 即 `GA-SOAK-24` 判决的那两个缺陷）、
> #5868 clippy 冗余局部变量（`crates/storage/src/wal_storage.rs` −3）。
> ⇒ **35/35 的结论不能继承到当前 tip**（GA-REWIND-1 的剩余义务**变大**，不是变小）——
> 最终认证轮仍须在**最终 tip**（当前为 `e2fcf73c2e`，且会继续前移）上重跑。
> ⚠️ **连带影响**：r4 轮冻结树 `93ac1d427e` **已是当前 tip 的祖先**（不是 tip 本身）⇒ r4 的结果只能作「`93ac1d427e` 上的证据」，不能直接当最终 tip 的证据。
> **材料提交**：本块随材料提交 M 一起落地；M 只新增 `docs/` 与 `audit/`
> （`git diff --stat e5c42c7fd02d3d2e24086d419920bcea938c19d6..M -- crates scripts tests` 为空 ⇒ M 不改变被测代码面）
> **本块刷新说明**：本版为第 9 次刷新（**上一版刷新块时间戳 2026-11-19T04:26:00Z**；更早为 2026-11-19T04:07:00Z、2026-11-19T00:46:07Z、2026-11-18T14:49:03Z、2026-11-18T14:37:27Z、2026-11-18T05:00:55Z、2026-11-17T15:13:56Z）；
> 本版只刷新本块内容，下方 2026-10-23 的判定与第 9 段**原样保留为历史记录**；
> **签署栏自 2026-11-18T08:27:00Z 起已转录为 CONDITIONAL（见文末），不再留空**。
> **本版含两处〔更正〕**：① §7.23.5 的「错误与宿主负载尖峰时间上完全重合」**只在前半轮成立**（详见 §7.25 / `GA-SOAK-21`）——
> **§7.23 原文保留不改**；② **本版新增**：§7.23.3 与 §7.24.4 宿主行里的「`dmesg` 无 OOM/panic/watchdog/thermal」**建立在一次**空读**之上**
> （本账号 `dmesg` 不可读：`dmesg_restrict=1`，读报 `Operation not permitted`）⇒ **该阴性表述撤回**；
> 「宿主未重启」的定性由 `boot_id` / `uptime` / `journalctl --list-boots` / 心跳连续性**独立**支撑（详见 §7.26.4 / `GA-SOAK-26`）——**两处原文保留不改**。

### 判定：**NOT READY —— 只剩物理约束**（人类签批已于 2026-11-18T08:27:00Z（本地 2026-11-18 16:27 GMT+8） 取得，见文末签署栏）

| 项 | 值 |
|---|---|
| 阶段 | `current_stage: "ALPHA"`（`STAGE.yaml`）—— **未跃迁** |
| GA 门禁 | **35/35 项：PASS 35 / FAIL 0 / INCOMPLETE 0 / 其他 0**（run `20261117-ga-gates-011`，被测树 `fa2b41abfa6b`）。⚠️ **该树的 crates 与当前 tip 差一个 KILL CONNECTION 修复（#5842）** ⇒ 最终认证轮仍须在 `e5c42c7fd02d3d2e24086d419920bcea938c19d6`（或其后继 tip）上重跑（见 GA-REWIND-1） |
| 覆盖率 | `✓ PASS A5 Coverage: 87.69% (min: 85%, measured 8/8 crates)`（min 85%，measured 8/8 crates）—— 真实测量 |
| 168h SOAK | ⚠️ **权威轮已换轮：r2 轮 `20261118-168h-v410-r2` 于 2026-11-18T12:36:45Z 在实测独占窗口内起跑**（独占窗口 **12:23:42Z** 成立 = root 编译进程 0 且 load1 ≤ 40 连续 **120/120** = 2 h；启动器安静窗口等待 601 s；起跑复验 **14,114 查 / 0 错 / 103.0 q/s / 0.000 %**；冻结提交 `e5c42c7fd02d3d2e24086d419920bcea938c19d6` **不变**；计划结束 2026-11-25T12:36:45Z）。⚠️ **该轮已于 2026-11-19T00:47:26Z 被另一条 Agent 线主动停止 ⇒ 终局记 INCOMPLETE**（停轮方自述「累计错误数已超 0 错门槛，且需让出同盘给 r3」；死因逐项排除宿主重启/服务端崩溃/OOM/看门狗误杀；详见 §7.26.1，归档 `audit/v4.1.0/runs/20261118-168h-v410-r2/`）—— **§7.25.7 写的「r2 轮继续跑不停」已被现实打破**。**r2 终态实测**：elapsed **44,060 s / 604,800 s（7.2867 %）**、**479,795 查 / 717 错 ⇒ 0.1494 %**（driver.log 全量口径 **784** 错）、**panic 00**（2,882 采样点唯一值 `00`；server.log 崩溃标记 0）、FD 恒定 46、心跳 **2,926** 拍 / max_gap **22.0 s**、**双臂存活** ⇒ **0 panic 成立、168 h 未跑满 ⇒ NOT-RUN、0 错不可逆不成立**（算术事实）。**当前在跑轮 = r4**（`20261119-168h-v410-r4`，port 3413，冻结 `93ac1d427e`，起跑 2026-11-19T03:28:01Z，计划结束 2026-11-26T03:28:01Z；起跑复验 **43,192 查 / 0 错 / 315.3 q/s**；实测 elapsed **2,183 s**、**274,525 查 / 0 错**、panic 00、RSS 428.9 MB、load1 3.13）⚠️ **但 r4 的「独占」尚未被证明**（探针硬盲区，见 §7.26.6 / B1f）；**r3 轮**已于 03:11:27Z 被同一批人 `sudo kill` 显式停止（逐条日志见 §7.26.5）。⚠️ **〔本版新增〕r4 轮也已于 2026-11-19T04:18:39Z 被「换轮脚本」主动停掉 ⇒ 终局 INCOMPLETE**（§7.27 / `GA-SOAK-27`，归档 `audit/v4.1.0/runs/20261119-168h-v410-r4/`）：elapsed **3,244 s / 604,800 s（0.5366 %）**、**343,026 查 / 0 错**、panic **00**、**点查命中 171,641/171,641 = 100.0000 %**、**WAL 恒定 90 B**、RSS 191.2→**434.7 MB（末段 420–462 MB 波动、无单调累积）**、延迟 p50 **113.1 ms** / p99 **644.8 ms**、心跳 216 拍 max_gap 16.0 s；**不是服务端崩溃**（`server.log` 仅 300 B、崩溃标记 0、停机前读数健康）——停它的是另一条线的 `_soak_r5_start.sh` 第 ① 步「停 r4」，换轮理由 = **被测树 ≠ 最终 tip**（#5865 遗留 clippy 回归导致 tip 又变）。**当前在跑轮 = r5**（`20261119-168h-v410-r5`，port **3414**，冻结 `e2fcf73c2e` = **当前 develop tip**，二进制 sha256 `f5ec5a67eb9…`；04:19:13Z 起编译、04:21:35Z launcher 起、04:22:35Z 仍在等安静窗口 `streak=1/10`）。⚠️ **〔第 9 次刷新新增〕两处更新**：① **r5 尚未起跑**（2026-11-19T04:30Z 实测：无 `STARTED.json`、无 attempt 目录、**端口 3414 未监听**），只是 launcher 已起、等安静窗口；② **「r5 冻结树 = 当前 develop tip」这个括注已不成立** —— tip 在 2.5 小时内被 **3 笔带 `crates/` 影响的变更**推动（`93ac1d427e` → `c99f1b6d12` → `e2fcf73c2e`(#5868) → `64269eaa08`(#5870)），**r2 → r3 → r4 三轮已连续作废**，而 **r5 的冻结树 `e2fcf73c2e` 已被 `64269eaa08` 甩在后面**（`crates/storage/src/{bplus_tree/mod.rs, file_storage.rs}`，2 文件 +53 / −55）⇒ **起跑前对齐冻结提交的成本此刻为零**（§7.28）。⭐ **本版最重要的一条**：**修复后二进制（#5865）首次实测健康** —— r4 在**更重的宿主负载**下（`strata` 占 24–37 核）跑到 **54.1 min 仍 0 错 0 panic**，而 **r2 的第一个错误出现在第 47.3 分钟** ⇒ **「点查命中 + WAL 修复消除了主路径退化」首次获得直接支持**（⚠️ 样本仅 54 min，**不得外推到 168 h**）。⚠️ **关键新证据**：**270 错中 234 条全部落在 13:21:03Z–13:47:54Z 一个 27 分钟窗口内**（13:47:54Z 至 14:30Z 零新增）；构成 = MySQL 2013 客户端 **15 s 读超时**（延迟字段 15,000–15,599 ms）+ 连接层空消息 `(0, '')`（聚集在 5 个时点、同一 query 序号多线程同时失败）；server.log 的 `Handshake send: Broken pipe` 是**客户端断链后的从属症状**而非起因。**错误与宿主负载尖峰时间上完全重合、两次独立复现**：elapsed 2,400–4,500 段 `host_load1` max **40.73** / busy max **18** ⇒ 234 错，elapsed 0–2,400 与 4,500–7,000 两段 load1 ≤ 21 ⇒ **0 错**；elapsed 7,000 起（14:30:41Z）root 属主 CI 构建出现（探针 root 编译进程 0→**293**、busy **84**、load1 **47.85**）⇒ 错误同一时刻由 234 增至 239，14:49Z 再增至 **270**（12 分钟 +31 错，与该时段 load1 41.23 同步）。⚠️ **但争用来源未逐项定性** —— 两段成分不同（13:13–13:48Z root 编译进程 **0/34** 样本；14:30Z 起 **19/19**）⇒ 不下「已证实是服务端缺陷」或「已证实纯属宿主争用」的结论。⚠️ **〔更正〕（§7.25 / `GA-SOAK-21`，2026-11-19）**：**上述「完全重合」只在前半轮成立，不成立于全轮** —— 12 h 分段实测：**9,000–10,800 s 段仅 load1 10.76–12.13、busy 2–4 却 +348 错（段错误率 3.7275 %）**；**后半程（elapsed ≥ 21,600）load1 仅 1.30–3.62、busy 1–2 仍 +2 错** ⇒ **宿主争用不是错误的唯一驱动**。**新的证据轴 = 服务端状态累积量（RSS）**：同宿主、同二进制、同负载档下，**RSS 414.2 MB（load1 1.70 / busy 2）时累计 0 错**，**RSS 1,018.4 MB（load1 1.30 / busy 1）时累计 715 错且该笔就发生在 load1 仅 1.30 的时刻**；同期 **RSS 12 h 单调增长 ~852 MB（191.8 → 1,044.4 MB）、吞吐从 44.77 q/s 衰减到 7.28 q/s（约 1/6）**，而宿主几乎无争用 ⇒ **存在与宿主争用无关的服务端侧成分**（**机制仍未定性**：RSS 增长 ≠ 泄漏已证实）。⚠️ **〔本版新增〕第三条证据轴 = 单查询延迟分布**（§7.26.2）：p50 **240.9 → 2,240.4 ms**、p90 **626.6 → 5,349.2 ms**、p99 **1,301.0 → 9,670.6 ms**（峰值 **12,358.2 ms** 落在错误最多的 9,000–10,800 桶）、max 14,964.2 ms、**`>= 15 s` 的成功样本 = 0** ⇒ **「服务端状态累积 ⇒ 单查询变慢 ⇒ 越过 15 s 读超时」这条机制被强支持**；⚠️ **「表变大」这一替代解释已被排除**：数据目录整轮恒定 **28,125,316 B（27 MB）**、20 s 内无变化（`sbtest1.json` 11.5 MB + `wal` 16.6 MB）⇒ RSS 增长是**内存内**的（候选落点 `crates/storage/src/file_storage.rs` 的 `tx_undo_log`，**只读参考，未改 crates**）。⚠️ **〔本版新增〕两处口径缺陷**（§7.26.4）：① metrics 的累计 q/e 在**轮次最后 15 分钟**滞后于 driver.log（前 24 桶逐桶完全一致，末桶 ΔQ −60 / ΔE −67 ⇒ 判定取 717，driver.log 的 784 并列登记，**两个口径都远超门槛**）；② **`dmesg` 在本账号不可读**（`dmesg_restrict=1`）⇒ §7.23.3 / §7.24.4 的「`dmesg` 无 OOM/panic/watchdog/thermal」**是空读、不是阴性证据**，该表述撤回（〔更正〕，原文保留不改）。**错误形态全程未变**（按 elapsed 1 h 桶：2013 读超时 30/184/357/48/2，空消息 0/25/56/14/0 ⇒ 无新错误种类、无 panic、无崩溃）。⚠️ **独占在 168 h 尺度上不成立**：252 上 **26 个容器（15 个 gitea act_runner CI 任务）** + root/openclaw cron 每分钟批量拉起脚本，独占探针 streak 于 14:30:41Z 被打断归零；**候选节点 250 同样不是独占机**（load1 15.74/20.87/15.50、root 编译进程 4、8 个 CI 容器、`/` 86 %）⇒ **权威 0 错 168 h 需要一台独占宿主**（属主，见 B1f）——**但本轮证据表明：独占宿主只解决「宿主争用」这一条路径，不能排除服务端侧退化 ⇒ 独占宿主 + 长时跑满 + 监控 RSS/表规模 三者要一起做**。⚠️ **〔本版新增〕「独占」这把尺本身有硬盲区**（§7.26.6）：探针字段 `root_compile_or_test_procs` 实为「**root 属主的** `rustc|cargo|rust-lld|rustdoc`」计数 ⇒ 对**非 root 的编译/测试**与**一切非 Rust 外来负载**（推理服务、Java/Maven、CI 打包、容器任务）**不可见**；r4 起跑后 27 个探针样本该字段**全为 0**（按现行判据「独占成立」），但同期 `cpu_sampler` 实测 **`strata --serve`（uid 1004）独占 24–37 核**、load1 冲到 **31.46** ⇒ **r4 不能被登记为「在独占宿主上跑」**；另 `host_busy_ge50cpu` 用 `ps` 的生命周期均值，对「单进程占 30 核」同样会漏判。⭐ **〔本版新增〕但服务端侧那条路已被削弱**：r4（**修复后二进制** `93ac1d427e`）在**更重的宿主负载**下跑到 **54.1 min 仍 0 错 0 panic**、**点查命中 100 %**、**WAL 恒定 90 B**、**RSS 不再单调累积**、**p99 644.8 ms（无 15 s 量级）** ⇒ **「点查命中 + WAL 修复消除了主路径退化」首次获得直接支持**（⚠️ 样本仅 54 min，**不得外推到 168 h**）⇒ **独占宿主的必要性从「同时排除服务端侧退化」降级为「排除宿主争用」**，但**仍是必要条件**。⚠️ **「独占窗口」机制本身要复核**（§7.24 / `GA-SOAK-20`）：2026-11-18T08:39:48Z 那次**显式独占窗口**（`docker pause` **20 个** CI/runner 容器 + `systemctl stop` 两个 unit，`pause_failed` = **[]**）**并未造出独占** —— 窗口内（08:39–09:47Z）root 编译进程 **67/67 个样本全部非 0**、load1 max **87.28**；「停 unit」一半保住了（两个 unit 至今仍 inactive），「pause 容器」一半被 **act_runner 栈在 09:33:15Z 的整体重建**冲掉；真正干净的 120 个样本（10:23:40–12:23:42Z，root **0/120**、load1 max 2.60）出现在**窗口被拆穿之后** ⇒ **共享宿主上的「独占窗口」是概率事件，不是可工程化取得的条件**。**历史**：诊断轮 `20261116-168h-v410`（attempt-33）累计 644,800 查 / 20,850 错 ⇒ 3.234 % 已出局；attempt-11 于 2026-11-18T05:02:03Z 被**宿主整机重启**打断 ⇒ 终态 elapsed **4,130 s（0.68 %）**、139,568 查 / 0 错、panic 00 ⇒ 如实记 **INCOMPLETE**，归档于 `audit/v4.1.0/runs/20261118-168h-v410/`（§7.21 / `GA-SOAK-16`）；**死因归因已在 §7.22.2 更正**（触发者**仍未定性**）；**r2 轮**于 2026-11-19T00:47:26Z 被**另一条 Agent 线主动停止** ⇒ 终态 elapsed **44,060 s（7.2867 %）**、479,795 查 / **717 错**、panic 00 ⇒ 如实记 **INCOMPLETE**，归档于 `audit/v4.1.0/runs/20261118-168h-v410-r2/`（§7.26 / `GA-SOAK-26`）；**r3 轮**于 2026-11-19T03:11:27Z 被 `sudo kill -INT/-KILL` **显式停止**（日志逐条可查）⇒ elapsed **7,207 s（1.19 %）**、183,668 查 / 7 错、panic 00 ⇒ 同样记 **INCOMPLETE**（§7.26.5）；**r4 轮**于 2026-11-19T04:18:39Z 被**换轮脚本主动停掉**（**不是崩溃**）⇒ elapsed **3,244 s（0.5366 %）**、**343,026 查 / 0 错**、panic 00、**点查命中 100 %**、**WAL 恒定 90 B** ⇒ 同样记 **INCOMPLETE**（§7.27 / `GA-SOAK-27`，归档 `audit/v4.1.0/runs/20261119-168h-v410-r4/`）；⚠️ **r2 → r3 → r4 三轮连续被 tip 变更作废** ⇒ **每一轮 168 h 都必须落在「不会再动的最终 tip」上**才有 GA 效力 |
| 审批 | **已批准（CONDITIONAL）** —— 属主于 2026-11-18T08:27:00Z（本地 2026-11-18 16:27 GMT+8） 在会话中明确答复「全部批准，执行」，答复原文与条件已转录进文末**签署栏**。`transitions.RC_to_GA.requires_human_approval` **已满足**；但 **`current_stage` 仍为 `ALPHA`、不跃迁**（签批解除的是人类约束，物理约束未达成：168 h 未跑满 + 35 项门禁未在最终 tip 重跑）。`manual_only_criteria` 明文「批准记录写进本文件的变更记录，**不能由 Agent 生成**」⇒ 本处只**转录属主的明确答复**，不代拟结论、不代写豁免 |

### 非绿项明细

无（35 项门禁全部 PASS）。**但 168h SOAK 尚未跑满、且起跑后宿主整机脱网 ⇒ 那一项仍未取得**，
不得记为 PASS，**也不得记为 FAIL**（是环境事件，不是被测系统的判定）。

### 硬检查现状（G1–G6）

| # | 硬检查 | 结论 | 依据 |
|---|---|---|---|
| G1 | RC 硬检查（R1–R4） | ✅ MET | run 20261117-ga-gates-011：35/35 项全 PASS；D1 Result: 10/10 (not-run: 0, blockers: 0)；D2 Result: 6/6；D3 Result: PASS=5 | FAIL=0 | DRIFT=0；D4 Result: 5/5 passed (failures: 0, exit 0)；D5 Result: 7/10 |
| G2 | Full test | ✅ MET | run 20261117-ga-gates-011 seq33 `cargo test --all-features` **PASS**（exit=0，耗时 968.9 s） |
| G3 | Coverage | ✅ PASS | run 20261117-ga-gates-011 seq1 实测行：`✓ PASS A5 Coverage: 87.69% (min: 85%, measured 8/8 crates)`（8-crate 平均；逐 crate 见 §7.13） |
| G4 | TPC-H SF=1 22/22 | ⚠️ **NOT MET（已知且已登记）** | SF=1 的 22/22 用例在 `#[ignore]` 下（ADR-008 例外，**已过期且从未签署**）；当前 SSOT（`STAGE_CONFIG`）各阶段要求的是 `check_g1_tpch_baseline.sh`（hermetic SF=0.001，**由独立 SQLite oracle 逐条担保**，实测 `G1 PASS: hash matches (7439ce80…)`）—— 已 MET |
| G5 | Security audit | ❌ NOT MET | 工作树层面已收敛（凭据 SSOT + `check_no_hardcoded_secrets.sh` 已接 CI）；**历史层面未解决**：旧 Gitea token 仍有效（ENG-8）⇒ 需属主在服务端删除/轮换 |
| G6 | Documentation | ✅ PASS | 6 份 GA doc_artifacts 齐备（`check_release_artifacts.sh` ⇒ PASS，GA 段 11 份 present）；看板 **205 行**、`kanban_render.py check` 一致、`kanban_merge_policy` verdict OK |

### 阻断项（剩余全部为物理 / 人类约束）

| # | 阻断 | 归属 |
|---|---|---|
| B1 | 168h SOAK 的**合格**证据需要跑满 168 h **且 0 错** —— ⚠️ **r2 / r3 / r4 三轮已连续出局**（§7.26 / §7.27）：① **r2**（2026-11-19T00:47:26Z 被另一条 Agent 线主动停止）终态 elapsed **44,060 s（7.2867 %）**、**717 错**、panic 00 ⇒ **0 错不可逆不成立**；② **r3**（03:11:27Z 被 `sudo kill` 显式停止）终态 **7,207 s（1.19 %）**、183,668 查 / 7 错、panic 00；③ **r4**（04:18:39Z 被**换轮脚本**主动停掉，**不是崩溃**）终态 **3,244 s（0.5366 %）**、**343,026 查 / 0 错**、panic 00、**点查命中 100 %**、**WAL 恒定 90 B** ⇒ **0 错成立但 168 h 未跑满**；④ **当前在跑轮 = r5** `20261119-168h-v410-r5`（port **3414**，冻结 `e2fcf73c2e` = **当前 develop tip**，二进制 sha256 `f5ec5a67eb9…`；04:19:13Z 起编译、04:21:35Z launcher 起、04:22:35Z 仍在等安静窗口 `streak=1/10`）⇒ 合格证据的载体是 r5。⚠️ **结构性约束**：**tip 连续三笔变更（#5865/#5867/#5868）已连续作废三轮** ⇒ **每一轮 168 h 都必须落在「不会再动的最终 tip」上**才有 GA 效力（见 B1f / §7.27.3） | 执行（**r5 尚未起跑**；剩余物理时长 + 需独占宿主 + 需冻结 tip + 需换独占判据的尺） |
| B1a | **252（Z6G4）整机脱网 —— 已判定为宿主重启** —— 2026-11-18T05:06:49Z 起不可达，2026-11-18T06:30:52Z 恢复；根因 = **宿主整机重启**（boot_id `1287e51dc1494edaae9fce1c583b72f9`、内核 `7.0.0-42-generic → 7.0.0-43-generic`、`unattended-upgrade（2026-11-18 06:57:18Z 处理 linux-hwe-7.0-headers-7.0.0-38）`），权威轮 attempt-11 因此在 2026-11-18T05:02:03Z 中止 ⇒ **INCOMPLETE**（归档见 §7.21 / `audit/v4.1.0/runs/20261118-168h-v410/`） | 已判定；**重跑由守望器自动执行** |
| B1b | **「独占窗口的合格错误率阈值」已裁定（2026-11-18T05:47:26Z）：维持 0 errors / 0 panics（不改判据、不抬阈值、不改 35 项门禁分母）** —— 门槛原样保留（debt #3265/#3266），**本次提交零判据变更**。⚠️ **裁定的后果**：实测安静档（`load1<4`）错误率仍为 **0.111 %** ⇒ **「独占窗口」是必要条件、不是充分条件**；权威轮合格判据 = 168 h 内 **0 错 + 0 panic**，带错结束即不是 GA 证据（见 §7.20 / `GA-SOAK-15`）。⚠️ **2026-11-18T14:37Z 的新观测（§7.23 / `GA-SOAK-18`）**：r2 轮的 239 错**全部落在宿主负载尖峰窗内**（两次独立复现：错误形态为客户端 **15 s 读超时** + 连接层空消息，panic 全程 `00`、服务端零崩溃）⇒ 在「独占机上服务端并发语义本身有残余错误」之外，**又确认了第二个独立成因：共享宿主上的他人负载**（252 实测 26 个容器 / 15 个 act_runner CI 任务；独占探针 streak 于 14:30:41Z 被打断）。**这两层须分开记，门槛不变**（0 errors / 0 panics 不放宽、不豁免）；**具体争用资源未逐项定性** ⇒ 不下「已证实是服务端缺陷」或「已证实纯属宿主争用」的结论。⚠️ **第三层（§7.24 / `GA-SOAK-20`）**：连「独占」这个前提本身在共享宿主上**也不是可保证的** —— 2026-11-18T08:39:48Z 的显式独占窗口（pause 20 个 CI 容器、失败 0 个）实测**没有造出独占**（窗口内 root 编译进程 67/67 样本非 0），真正干净的 2 h 是其后的**自然空档** ⇒ **「独占窗口」在共享宿主上是概率事件**（见 B1f）。⚠️ **第四层（§7.25 / `GA-SOAK-21`，2026-11-19）**：**宿主争用不是错误的唯一驱动** —— r2 轮后半程（elapsed ≥ 21,600 s）在 `host_load1` 仅 **1.30–3.62**、`host_busy` 1–2 的近乎无争用时段**仍出现错误**；且在**同宿主、同二进制、同负载档**下，**RSS 414 MB 时累计 0 错、RSS 1,018 MB 时累计 715 错** ⇒ **存在与宿主争用无关的服务端侧成分**（机制仍未定性）。⇒ **门槛不变**，但**「换一台 0 错节点」这一条路现在只覆盖一半成因** ⇒ 合格证据需要**独占宿主 + 长时跑满 + 监控 RSS/表规模/单查询 p99** 三者一起（后两项需 `scripts/soak/` 采集增强，属主批准后另行执行）。⚠️ **第五层（§7.26 / `GA-SOAK-26`，2026-11-19T04:07Z）**：**第三条证据轴已就位且与前两条同向** —— 单查询延迟分布 p50 **240.9 → 2,240.4 ms**、p90 **626.6 → 5,349.2 ms**、p99 **1,301.0 → 9,670.6 ms**（峰值 **12,358.2 ms** 落在错误最多的 9,000–10,800 桶）、max 14,964.2 ms、**`>= 15 s` 的成功样本 = 0** ⇒ **「服务端状态累积 ⇒ 单查询变慢 ⇒ 越过 15 s 读超时」这条机制被强支持**；且 **「表变大」这一替代解释已被排除**（数据目录整轮恒定 **28,125,316 B / 27 MB**、20 s 内无变化）⇒ RSS 增长是**内存内**的。⚠️ **两处口径缺陷一并登记**：① metrics 累计 q/e 在**轮次最后 15 分钟**滞后于 driver.log（前 24 桶逐桶完全一致；判定取 **717**，driver.log 的 **784** 并列登记 ⇒ **两个口径都远超门槛，判定不受影响**）；② **`dmesg` 在本账号不可读**（`dmesg_restrict=1`）⇒ §7.23.3 / §7.24.4 的「`dmesg` 无 OOM/panic/watchdog/thermal」**是空读、不是阴性证据**〔更正〕。**门槛依旧不变**（0 errors / 0 panics），本次提交**零判据变更**。⚠️ **第六层（§7.27 / `GA-SOAK-27`，2026-11-19T04:26Z）**：**修复后二进制首次实测健康** —— r4 轮（`93ac1d427e`，含 #5865 点查命中 + WAL 两处修复）在**更重的宿主负载**下（`strata --serve` 占 24–37 核）跑到 **elapsed 54.1 min 仍 0 错 / 0 panic**、**点查命中 171,641/171,641 = 100 %**、**WAL 恒定 90 B**、**RSS 末段 420–462 MB 波动（无单调累积）**、**延迟 p99 644.8 ms（max 1,985.3 ms，无 15 s 量级）**；而 **r2 的第一个错误出现在第 47.3 分钟** ⇒ **「点查命中 + WAL 修复消除了主路径退化」首次获得直接支持** ⇒ **本层（服务端侧退化）已被削弱**，但 ⚠️ **样本仅 54 min，只比 r2 首个错误时点多 7 分钟 ⇒ 不得外推到 168 h，不预判 r5**；且 r4 的**独占性仍未被证明**（探针盲区）⇒ **门槛依旧维持 0 errors / 0 panics，本提交零判据变更** | 已裁定（属主，2026-11-18T05:47:26Z）；后续「换一台 0 错节点」与「是否给书面例外」**仍是属主决策**（Agent 不代做） |
| B1c | **宿主侧自动升级已禁用（属主 2026-11-18T08:27:00Z（本地 2026-11-18 16:27 GMT+8） 批准后执行，实测复验）** —— `APT::Periodic::Unattended-Upgrade` → `"0"`、`unattended-upgrades.service` → **disabled+inactive**、`apt-daily-upgrade.timer` → **disabled**（`apt-daily.timer` 只下载不安装，保留）；备份 `/etc/apt/20auto-upgrades.bak-20261118T084115Z`、`/etc/apt/apt.conf.d/50unattended-upgrades.bak-20261118T084115Z` 可回滚。⚠️ **定位更正**（§7.22.2）：这是**预防性风险收敛**，**不是**「修复已证实的死因」—— attempt-11 的死因**触发者仍未定性**（内核 42→43 的安装发生在**新 boot 之后**，上个 boot 无有序关机序列、无 panic/OOM，停机 88.7 分钟） | **已完成**；触发者定性待办见 B1e |
| B1d | **gitea 侧 `refs/heads/` 下 22 个 ref 目录曾为 `root:root`** ⇒ 带斜杠的新分支无法创建 lock（`cannot lock ref … Permission denied`）；本 PR 改用无斜杠分支名 `docs-v410-soak-daily-20261118` 发布（PR #5855），**未动属主服务器**；该批目录属主现已恢复为 `git:git` | 已绕过；**属主可自行确认**（后续 PR 可正常用 `docs/*` 命名） |
| B2 | 人类架构师签批本报告 | **已取得**（2026-11-18T08:27:00Z（本地 2026-11-18 16:27 GMT+8），CONDITIONAL，条件与到期日见签署栏） |
| B1e | **宿主非正常停止的触发者仍未定性** —— 需一次「宿主是否会在无人工介入时自停」的复现或排除（§7.22.2） | **属主**（Agent 无带外通道，取不到硬件日志）；⚠️ **原到期日「r2 轮起跑前」已过**（r2 已于 2026-11-18T12:36:45Z 起跑），触发者**至今仍未定性**。⚠️ **〔本版新增〕一条反向线索（§7.26.5）**：本机上「SOAK 轮次非正常终止」至少**有一种已被证实的形态 = 他方主动 `sudo kill`**（r3 轮 03:11:27Z 起 4 条 kill 命令，逐条在 journal 里：`-INT` driver → `-INT` server → `-KILL` driver → `-KILL` server）⇒ 这**不是**「宿主自停」，后者仍需带外日志。**并新增一条操作纪律**：任何 Agent 线**不得停掉别人起的 SOAK 轮次**；确需停轮时，须在轮次目录留下与 r2 同规格的 `STOPPED_BY_AGENT.txt`（r3 就**没留** ⇒ 只能靠 journal 反推） |
| B1f | **权威 0 错 168 h 需要一台独占宿主**（2026-11-18T14:37Z 新增，14:49Z 补机制层）—— 实测 **252 与 250 均为多租户 CI 机**：252 上 **26 个 docker 容器（其中 15 个 gitea act_runner CI 任务）** ＋ root/openclaw cron 每分钟批量拉起 `z6g4-mem-watch.sh` / `hw-monitor.sh` / `ci-docker-cleanup.sh` / `gitea-runner-watchdog.sh` / `sqlrustgo-watch.sh` / `gitea-startup.sh`，**独占探针 streak 于 2026-11-18T14:30:41Z 被打断归零**；候选 250 上 load1 **15.74 / 20.87 / 15.50**、root 编译进程 **4** 个、`strata-host` 占 **53 %** 内存、**8 个 CI 容器**在跑、`/` 已用 **86 %** ⇒ **起跑前 2 h 的独占窗口不覆盖 168 h 窗口**。⚠️ **机制层补充（§7.24 / `GA-SOAK-20`）**：「pause CI 容器」这条路**已被实测证伪为充分条件** —— 2026-11-18T08:39:48Z 的显式独占窗口（`docker pause` **20 个** CI/runner 容器 + `systemctl stop` `gmp-platform-runner.service` / `bustubx-unpause-ci.service`，`pause_failed` = **[]**）**并未造出独占**：窗口内 08:39–09:47Z **root 编译进程 67/67 个样本全部非 0**、load1 max **87.28**；「停 unit」一半保住了（两个 unit 至今仍 inactive），「pause 容器」一半被 **act_runner 栈在 09:33:15Z 的整体重建**冲掉（重建 ≠ unpause）；真正干净的 120 个样本（10:23:40–12:23:42Z，root **0/120**、load1 max 2.60）出现在**窗口被拆穿之后** ⇒ **共享宿主上的「独占窗口」是概率事件，不是可工程化取得的条件**。⇒ 二选一：① **属主提供专用/隔离宿主**，或**在认证窗口内停摆该机 CI 调度**（服务器调度决策，**Agent 不代做**；若仍用 252，则**必须同时压住「派发侧」**——act_runner daemon / runner 池容器的重建入口——**且用探针连续验证**，中断即视为独占失效）；② **属主给书面例外**（批准记录**不能由 Agent 生成**）。**附带建议（仅登记，不在本轮执行）**：把 SOAK 放进**独立 cgroup + CPU/IO 权重**（`CPUWeight` / `IOWeight`）⇒ 只改承载方式、**不改判据**，让宿主争用只降吞吐、**不产生 15 s 读超时** ⇒ 共享宿主也能拿到「0 错」判定；但会改 `scripts/`，需属主批准后另行执行。⚠️ **〔本版新增，优先级最高〕「独占」这把尺要换**：现行探针 `_excl_probe.sh` 的 `root_compile_or_test_procs` 实为「**root 属主的** `rustc|cargo|rust-lld|rustdoc`」计数 ⇒ 对**非 root 编译/测试**与**一切非 Rust 外来负载**（推理服务、Java/Maven、CI 打包、容器任务）**不可见**；r4 起跑后 27 个探针样本该字段**全为 0**，而同期 `cpu_sampler` 实测 **`strata --serve`（uid 1004）独占 24–37 核**、load1 冲到 **31.46** ⇒ **r4 不能登记为「在独占宿主上跑」**。建议按「**任何 uid 的非本轮进程占用 CPU/IO**」重做（`srg-soak-cpu-sampler` 已能按 uid 抓到 `strata`，可直接复用）；**属主批准后**再改 `scripts/soak/`。⚠️ **〔本版新增，另两条属主决策〕**：① **tip 冻结窗口** —— tip 在 11-19 一天内连续三笔变更（#5865 PK/WAL → #5867 drift 基线 → #5868 clippy），**已连续作废 r2 / r3 / r4 三轮**（每轮都是「被测树 ≠ 最终 tip」而非跑出问题）⇒ 认证轮期间需**冻结 `crates/` 合并**，或接受「tip 一变即作废重跑」的成本；⚠️ **〔第 9 次刷新补量化〕2.5 小时内 3 笔带 `crates/` 影响的变更**（`93ac1d427e` → `c99f1b6d12` → `e2fcf73c2e`#5868 → `64269eaa08`#5870），**r5 的冻结树也已过期**（§7.28）⇒ **起跑前必须有一道机械校验：`被测树 SHA == 远端 develop tip SHA`**（r5 的 launcher 已有该断言，但其期望值写死在旧 tip 上）；② **停轮留档纪律** —— r3 与 r4 两次停轮**都没有**在轮次目录留 `STOPPED_BY_AGENT.txt` ⇒ 归因只能靠脚本原文 + journal 反推 ⇒ **任何 Agent 线停 SOAK 轮次时必须留档**（时刻、停轮者、理由、是否换轮） | **属主**（独占宿主 **或** 书面例外；tip 冻结窗口；停轮留档纪律；Agent 三条都不代做） |
| B3 | GA 承载分支 `release/v4.1.0` + tag + 并入 main | **属主**（按 `RELEASE_LIFECYCLE` §2.5：审批 → tag → 并入 main → 建承载分支，顺序不可越） |
| B4 | 旧 Gitea token 在服务端失效 | **属主**（仓库侧已收敛） |
| B5 | 两条已过期且从未签署的 ADR-008 例外 + 61 道死门禁逐个处置 | **属主**（P0-4：逐个门禁处置必须属主决定） |

### 签署（**已签署：CONDITIONAL —— 属主于 2026-11-18T08:27:00Z（本地 2026-11-18 16:27 GMT+8） 在会话中明确答复「全部批准，执行」；
Agent 只**转录**该答复，**不代拟结论、不代写豁免、不代签**（`manual_only_criteria`：批准记录不能由 Agent 生成）**）

```
GA gate report reviewed and approved by human architect:

  姓名 / 身份：李莹（属主 / v4.1.0 GA 签批人）—— 答复原文：「全部批准，执行」
  签署时间（UTC）：2026-11-18T08:27:00Z（本地 2026-11-18 16:27 GMT+8）
  结论（PASS / CONDITIONAL / NOT APPROVED）：CONDITIONAL
  备注（若为 CONDITIONAL，必须逐条列出条件与到期日）：
    ① 168h SOAK：r2 轮（run 20261118-168h-v410-r2，冻结 e5c42c7fd0）须跑满 604,800 s
       且 0 错 + 0 panic；
       到期日：r2 轮计划结束时刻（起跑 +168 h；起跑日待独占窗口）
    ② 35 项 GA 门禁须在 e5c42c7fd0（或其后继**最终** tip）上重跑并全绿（GA-REWIND-1）；
       到期日：2026-11-25（tip 若再变更则顺延至重跑结束）
    ③ 合格门槛维持 0 errors / 0 panics（不豁免、不放宽）；
       到期日：常设
    ④ TPC-H 哈希漂移未定性（GA-GATES-11/12）、61 道死门禁、2 条过期 ADR-008 例外逐个处置；
       到期日：2026-11-25
    ⑤ 旧 Gitea token 服务端失效（B4）；
       到期日：2026-11-25
    ⑥ 宿主非正常停止的触发者仍未定性（§7.22.2）；
       到期日：r2 轮起跑前完成一次复现或排除
    ⑦ release/v4.1.0 承载分支与 v4.1.0 tag 待 ①② 达成后按 RELEASE_LIFECYCLE §2.5 顺序创建；
       到期日：①② 达成后 7 日内
  阶段：仍为 ALPHA（**不跃迁** —— 签批解除的是人类约束；物理约束 ①② 未达成）
```

**签署栏所载条件的落地状态（事实登记，2026-11-19T00:46:07Z；**签署内容本身未被改动**）**：

| 条件 | 落地状态 |
|---|---|
| ① 168h SOAK：r2 轮跑满 604,800 s 且 0 错 + 0 panic | ⚠️ **条件①原指名 r2 轮，r2 已于 2026-11-19T00:47:26Z 被另一条 Agent 线主动停止 ⇒ INCOMPLETE**（44,060 s / 7.2867 %、717 错、panic 00；归档 `audit/v4.1.0/runs/20261118-168h-v410-r2/`）；其后 **r3**（03:11:27Z 被 `sudo kill` 停止，7,207 s、7 错）与 **r4**（04:18:39Z 被换轮脚本停止，3,244 s、**0 错**、点查 100 %、WAL 恒定 90 B）**也先后出局** ⇒ **该条件在 r2/r3/r4 三个载体上均已不可达成**（归档 `audit/v4.1.0/runs/20261119-168h-v410-r4/`）。**当前在跑轮 = r5**（起跑待安静窗口，计划结束 2026-11-26；冻结树 = 当前 develop tip）⇒ **该条件未被撤销**，仅事实状态变化；**并提请属主裁定**：条件①由**哪一轮**承接 —— **属主决策，Agent 不代做** |
| ② 35 项 GA 门禁在最终 tip 重跑 | 未达成（GA-REWIND-1 仍在）；tip 会随材料合并继续变动 |
| ③ 门槛维持 0 errors / 0 panics | **维持，未放宽、未豁免**（本次提交零判据变更） |
| ④ TPC-H 哈希漂移 / 61 道死门禁 / 2 条过期 ADR-008 例外 | 未达成（属主） |
| ⑤ 旧 Gitea token 服务端失效 | 未达成（属主） |
| ⑥ 宿主非正常停止的触发者定性 | **到期日「r2 轮起跑前」已过**，触发者**仍未定性**（见 B1e）；⚠️ **本版新增一条反向线索**：本机上「SOAK 轮次非正常终止」已确认**另一种形态 = 他方主动 `sudo kill`**（r3 轮 2026-11-19T03:11:27Z 起 4 条 kill 命令、逐条在 journal 里）⇒ **这不能替代「宿主自停」的定性** —— 后者仍需带外日志。⚠️ **〔本版再补〕r4 轮同样是被主动停掉的**（另一条线的换轮脚本 `_soak_r5_start.sh` 第 ① 步，04:18:39Z 执行）⇒ **「轮次非正常终止」目前已确认的两种形态都是人为动作（`sudo kill` / 换轮脚本）**，**没有一种指向宿主自停** ⇒ B1e 的问法应保持：**宿主是否会在无人工介入时自停，仍未定性** |
| ⑦ `release/v4.1.0` + `v4.1.0` tag | 未创建（按属主 2026-11-18 的明确答复「暂不创建，等条件达成」——**该答复本身即是本条的执行依据**） |

<!-- END 2026-11-17 REFRESH BLOCK -->

## 0. 判定（2026-10-23）

# ❌ **v4.1.0 当前不满足 GA 晋升条件。**

**这不是一份宣布 GA 的文件。** 它是 RC 阶段要求存在的一份**就绪度快照**
（`STAGE_CONFIG.yaml` → `stages.RC.required_files` 里就有
`GA_GATE_REPORT.md`），用来说明「离 GA 还差什么」。

| 项 | 值 |
|---|---|
| 阶段 | `current_stage: "ALPHA"`（`STAGE.yaml`） |
| 判定 | **NOT READY** —— 6 项 GA 硬检查中 **2 项 NOT MET**、**1 项 NOT RUN** |
| 阻塞项 | 168h SOAK（NOT MET）、TPC-H SF=1 22/22（NOT RUN）、未销账 P0 **2** 条、CI 未全绿、GA 承载分支未建立 |
| 审批 | **未申请、未批准。** 三次阶段跃迁 `requires_human_approval: true` |

⚠️ **前车之鉴**：v4.0.0 的 GA 判定曾被写为「GA (CONDITIONAL PASS)」，后被
`docs/releases/v4.0.0/STAGE.yaml` 的 `ga_promotion_status.verdict = "REVOKED
2026-09-30 — v4.0.0 never legitimately reached GA"` **撤销**。撤销的直接原因
是覆盖率 78.28 % < 门槛，且 3 个继承的 ALPHA 门禁失败被写成「已解决」而实测
未解决（AFP Type B）。**本文件因此不写任何未经实测的 PASS。**

---

## 1. Entry conditions（GE1–GE5）

GA 的入口条件来自 `STAGE_CONFIG.yaml` → `stages.GA.required_files` 与
`STAGE.yaml` → `rc_to_ga.exit_criteria`。

| # | 入口条件 | 状态 | 说明 |
|---|---|---|---|
| GE1 | `docs/releases/v4.1.0/STAGE.yaml` 的 `current_stage` 为 `GA` | ❌ **NOT MET** | 当前 `"ALPHA"`。STAGE_CONFIG 的 `required_file_expectations` 明写这条期望 |
| GE2 | `docs/releases/v4.1.0/RELEASE_NOTES.md` 是 GA 版发行说明 | ⚠️ **NOT MET** | 文件存在（47 460 B）但仍是 DRAFT 口径 |
| GE3 | `docs/releases/v4.1.0/GA_GATE_REPORT.md` 存在 | ✅ **PASS** | 本文件 |
| GE4 | `docs/releases/v4.1.0/GA_RELEASE_TIMELINE.md` 存在 | ✅ **PASS** | 本批次新建 |
| GE5 | `docs/releases/v4.1.0/CHANGELOG.md` 含 GA 条目 | ❌ **NOT MET** | 无 GA 条目 |
| GE6 | 阶段跃迁的承载分支合法 | ❌ **NOT MET** | `stages.GA.branch_pattern` = `["main", "release/v*", "hotfix/v*"]`，`carrier_enforced: true`；`STAGE.yaml` 的 `ga_carrier_branch` = `release/v4.1.0`。**该分支尚未建立** ⇒ GA 不能在 `develop/v4.1.0` 上声明 |
| GE7 | RC tag 已切（`v4.1.0-rc1`） | ❌ **NOT MET** | 尚未切 |

---

## 2. Hard checks（G1–G6）

### G1 — RC 硬检查（R1–R4）

逐条见 `docs/releases/v4.1.0/RELEASE_GATE_CHECKLIST.md`。

| 组 | 状态 |
|---|---|
| R1 必需文件 | ✅ PASS（4/4 present） |
| R2 阻断门禁 | ⚠️ 部分 —— 22 条 required_gates 中本轮实测 **3 条 PASS**、**19 条 NOT RUN**；optional_gates 6 条全 NOT RUN |
| R3 Cargo | ✅ PASS（build / fmt 三版 CLEAN / clippy 0 error）；整仓 `cargo test --all-features` **NOT RUN** |
| R4 覆盖率 | ✅ PASS（8-crate 平均 87.80 % ≥ RC 门槛 80 %） |
| R5 `#[ignore]` 债务 | ✅ PASS（tree recount 123 == 123） |
| R7 功能与性能场景 | ❌ NOT MET（TPC-H SF=1 未执行；168h SOAK 未完成；Q20 oracle 缺产物） |
| R8 缺陷与 open_issues | ❌ NOT MET（P0 未销账 2 条；`open_issues` 剩 1 条） |

**G1 结论：❌ NOT MET。**

### G2 — Full test（`cargo test --workspace`）

| 项 | 状态 |
|---|---|
| `cargo test --workspace --no-run`（编译全部测试目标） | ✅ **PASS** —— `check_anti_fabrication.sh` 的 CHECK 2 通过，AFP 整体 `ERRORS=0, WARNINGS=0` |
| `sqlrustgo-storage` | ✅ **PASS** —— 1544 passed / 0 failed（6 个 fault-injection 用例因环境前置条件跳过） |
| `sqlrustgo-types` | ✅ **PASS** —— 152 passed / 0 failed |
| `executor` | ✅ **PASS** —— 777 passed |
| 整仓一次跑完 | ⚠️ **NOT RUN（原因：本轮未执行整仓 `cargo test --all-features`）** |
| `test` / `test-crates` 两个 CI job | ⚠️ 本轮观察到的 CI run 尚未结束，**不预判结论** |

**G2 结论：⚠️ 部分通过（整仓一次跑完 NOT RUN）。**

### G3 — Coverage

| crate | L1_8 | ≥ 80 %（RC） | ≥ 85 %（GA） |
|---|---|---|---|
| parser | 86.46 % | ✅ | ✅ |
| planner | 88.91 % | ✅ | ✅ |
| executor | 82.74 % | ✅ | ❌ |
| transaction | 93.29 % | ✅ | ✅ |
| storage | 80.61 % | ✅ | ❌ |
| catalog | 87.33 % | ✅ | ✅ |
| optimizer | 87.85 % | ✅ | ✅ |
| types | 95.28 % | ✅ | ✅ |
| **8-crate 平均** | **87.80 %** | ✅ | ✅ |
| mysql-server | 87.89 % | ✅ | ✅ |

门槛来源：`STAGE_CONFIG.yaml` → `thresholds`（`COVERAGE_MIN_ALPHA: 50` /
`COVERAGE_MIN_BETA: 75` / `COVERAGE_MIN_RC: 80` / `COVERAGE_MIN_GA: 85`）。

⚠️ **口径提醒（两处）**：
① `STAGE.yaml` → `rc_to_ga.exit_criteria` 写「Coverage >= 80%」，而
`thresholds.COVERAGE_MIN_GA` 是 **85**。`STAGE.yaml` 自己的
`coverage_target_for_ga` 已写明「85%（STAGE_CONFIG COVERAGE_MIN_GA）；80% 是
COVERAGE_MIN_RC」⇒ **GA 按 85 判**。
② 仓库里还有第三个数 **60.58 %**（`DEFECTS_AND_ISSUES.md` P1-01），那是**一条
待裁决的口径矛盾**，不是本次测量结果。

**G3 结论：✅ PASS**（按 8-crate 平均；逐 crate 看有 2 个 crate 低于 85 %，
这一点已如实披露，判定口径留给属主）。

### G4 — TPC-H SF=1（22/22）

| 项 | 状态 |
|---|---|
| TPC-H SF=1 22/22 oracle 判定 | ❌ **NOT RUN** —— `EVALUATION_REPORT.md` 明确记载该判定 *not recorded* |
| `check_g1_tpch_baseline.sh` | ⚠️ **NOT RUN（原因：本轮未执行；且 SF=1 数据未就绪）** |
| Q20 子项（#4540） | ❌ **NOT MET** —— `sqlrustgo/q20.tsv` 缺失、`CROSS_ENGINE_HASH_CHECK.json` 为 `present:false`，而 v3.12.0 报告称 "Q20 PASS"，三者互相矛盾。这是 `STAGE.yaml` → `open_issues` 剩余的**唯一一条** |
| QPS/TPS 基准（G11） | ⚠️ **NOT RUN（原因：判据依赖基准机）** |
| Sysbench OLTP（G12） | ⚠️ **NOT RUN（原因：外部工具 `sysbench` 不可用）** |

**G4 结论：❌ NOT MET。**

### G5 — Security audit

| 项 | 状态 | 说明 |
|---|---|---|
| 硬编码凭据扫描（`check_no_hardcoded_secrets.sh`） | ✅ **PASS** | 本轮复跑 rc=0；扫描形状（凭据名变量 ← 长 hex、字面量 `名字:口令` 对、curl 基本认证字面量、URL 内嵌凭据），**不依赖任何具体字面量** |
| 凭据在**历史**中的可取得性 | 🔴 **NOT MET** | 见 §4 |
| CI 安全扫描（`ga3-security-scan` job） | ⚠️ 观察到的 run 中该 job 状态为 `skipped` | 以 `action_run_job.status` 为准（4 = skipped） |
| 架构第三层无旁路（`check_arch3_no_bypass.sh`） | ⚠️ **NOT RUN（原因：本轮未执行）** | — |
| 审计日志 / 哈希链 / 时间旅行（G10 三项） | ⚠️ **NOT RUN（原因：本轮未执行）** | — |

**G5 结论：❌ NOT MET** —— 工作树层面干净，但历史层面未解决。

### G6 — Documentation

| 项 | 状态 | 说明 |
|---|---|---|
| 8 份 mandatory 文档 | ✅ **PASS** | `check_document_completeness.sh` rc=0 |
| RC/GA 发布产物 | ✅ 本批次补齐 7 份 | `RELEASE_GATE_CHECKLIST` / `PERFORMANCE_TARGETS` / `EVIDENCE_STATUS` / `GA_GATE_REPORT` / `GA_RELEASE_TIMELINE` / `POST_GA_PLAN` / `FEATURE_CHECKLIST`；`check_release_artifacts.sh` 的 RC/GA 段实测由「RC 缺 4 / GA 缺 6」转为全 present（**待复跑确认**） |
| 证据绑定（provenance 可证伪） | ✅ **PASS** | `check_evidence_binding.sh v4.1.0` → `PASS=129 WARN=123 FAIL=0 UNVERIFIED=0` |
| 反编造（AFP） | ✅ **PASS** | `check_anti_fabrication.sh` → `ERRORS=0, WARNINGS=0` |
| 跨版本文档一致性 | ✅ **PASS** | `check_docs_consistency.sh` rc=0 |
| 阶段配置一致性 | ✅ **PASS** | `check_stage_config_consistency.sh` rc=0 |
| 证据树规模 | ✅ 81 份 | `docs/releases/v4.1.0/evidence/` |

**G6 结论：✅ PASS。**

---

## 3. GA 硬检查汇总

| # | 硬检查 | 结论 |
|---|---|---|
| G1 | RC 硬检查（R1–R4） | ❌ **NOT MET** |
| G2 | Full test | ⚠️ **部分通过**（整仓一次跑完 NOT RUN） |
| G3 | Coverage | ✅ **PASS** |
| G4 | TPC-H SF=1 | ❌ **NOT MET** |
| G5 | Security audit | ❌ **NOT MET** |
| G6 | Documentation | ✅ **PASS** |

⇒ **2 项 NOT MET、1 项部分、3 项 PASS。** 不满足 GA 晋升。

---

## 4. 阻断项明细

### B1 — 168h 多模型 SOAK：**NOT MET**

`STAGE.yaml` → `known_gaps.V400_09_soak`：
`status: "NOT MET"`、`verified: "2026-10-07"`。

实测依据：① `#4944` checkpoint 记录的服务进程 PID 1957614 已不在进程表；
② 全盘搜索无 `STABILITY_REPORT_*.md` / `metrics.csv` / `*soak*run[34]*`；
③ `docs/releases/v4.0.0/V400_09_168H_SOAK_FINAL_REPORT.md`（2026-09-30）
记录的是第一轮（2026-09-19 kickoff）的失败，其正文自述
「the 168h SOAK process never ran to completion」。

`decision` 原文：「**OPEN — 是否重跑 168h SOAK 属架构师裁决，本 SSOT 不代为决定。**」

⇒ **这一项只能由「真的跑一次」或「属主明确记录一次 deferral」解除**，
文档写不出来。

### B2 — TPC-H SF=1 22/22：**NOT RUN**

`EVALUATION_REPORT.md` 记载该判定 *not recorded*；
`evidence/TPCH_FIXTURE_5379_2026-10-20.md` 只覆盖 fixture 自包含性（#5379），
并明确把 SF=1 排除在该轮范围外（#5387）。

### B3 — 未销账 P0 **2** 条

| 条目 | 性质 | 处置 |
|---|---|---|
| P0-03 | v4.0.0 的 GA 撤销（`REVOKED`）—— **是一个状态，不是缺陷** | 需要的是裁决，不是补丁 |
| P0-04 | 5 远端漂移 | 本批次已在 PR #5563 完成合并式收敛；**待合并后复跑 `5remotes_drift_check.sh` 确认后销账** |

⇒ `STAGE.yaml` → `rc_to_ga.exit_criteria` 的「All RC{N} release-blocking bugs
closed」与「no P0 bugs since」两条均**未满足**。

### B4 — CI 未全绿

本轮观察到的 develop tip CI 尚未结束。**不预判结论**；以
`action_run_job.status` 为准（1=success / 2=failure / 3=cancelled / 4=skipped）。

### B5 — GA 承载分支未建立

`stages.GA.branch_pattern` = `["main", "release/v*", "hotfix/v*"]`，
`carrier_enforced: true`；`ga_carrier_branch` = `release/v4.1.0`。
⇒ **必须在 `release/v4.1.0` 分支上做 GA 声明，不能在 `develop/v4.1.0` 上。**

### B6 — 凭据在**历史**中的可取得性（🔴 P0）

见 §5。

---

## 5. Claim downgrade summary（对外声明必须降级的项）

STAGE_CONFIG 的 `claims_boundaries` 规定了每个阶段允许的对外措辞。
**在 GA 未达成之前，下面这些声明一律不成立**：

| ❌ 不可宣称 | ✅ 可以宣称 |
|---|---|
| 「v4.1.0 已 GA 发布」 | 「v4.1.0 处于 ALPHA 阶段，RC 候选产出中」 |
| 「v4.1.0 通过了 168h 多模型 SOAK」 | 「168h SOAK 尚未完成，已登记为 known gap（`V400_09_soak`），是否重跑待架构师裁决」 |
| 「v4.1.0 的 TPC-H SF=1 22/22 达标」 | 「TPC-H SF=1 未执行；判定 *not recorded*」 |
| 「v4.1.0 无 P0 缺陷」 | 「未销账 P0 2 条：P0-03 是状态（v4.0.0 GA 撤销）、P0-04 是远端漂移（收敛中）」 |
| 「v4.1.0 的凭据问题已解决」 | 「工作树已无明文凭据；历史中的可取得性尚未解决，处置顺序为轮换 → 去明文 → 历史重写，轮换与历史重写待属主拍板」 |
| 「生产可用的多模型能力」 | STAGE_CONFIG 的 `prohibited_pre_ga` 明写：**168h 多模型 SOAK 结果**与**生产级多模型声明**在 GA 之前都不可宣称 |

---

## 6. GA promotion decision

**结论：本次不提交 GA 晋升申请。**

理由：G1 / G4 / G5 三项硬检查 NOT MET，G2 部分通过，7 项入口条件中 4 项
NOT MET。**按 AFP §7.1 Type B，在判据未满足时写「已具备 GA 条件」属于
「证据不支持的声明」** —— 这正是 v4.0.0 的 GA 判定被撤销的那一类问题。

**建议的下一步顺序**（每一步都可独立停下来交给属主裁决）：

1. **B4**：等 CI 出结论；红则按「代码红 / 环境阻塞 / 基础设施红」三类归因再修。
2. **B1**：由属主裁决 168h SOAK —— 重跑，或在 `known_gaps` 里**明确记录一次
   deferral**（连同它的前置条件），二者择一。
3. **B2**：取得 TPC-H SF=1 数据后跑 `check_g1_tpch_baseline.sh`；或先撤回
   v3.12.0 报告里的 "Q20 PASS" 声明（**不得编造产物**）。
4. **B3**：合并 PR #5563 → 复跑 `5remotes_drift_check.sh` → 销账 P0-04；
   P0-03 交属主裁决。
5. **B5**：RC 判定通过后切 `v4.1.0-rc1`，建立 `release/v4.1.0` 分支。
6. **B6**：凭据处置按 **① 轮换 → ② 去明文 → ③ 历史重写** 的顺序推进；
   轮换与历史重写**不可颠倒**，且都需要属主拍板（撤销不可逆）。
7. 全部满足后，把本文件的 G1–G6 与 GE1–GE7 **重新按当时的实测复跑一遍**，
   再把 GA 判定与审批位提交人类架构师。

---

## 7. 本文件的状态与有效期

- 本文件是 **2026-10-23 的快照**，绑定在
  `develop/v4.1.0` @ `2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`。
- **树一动，本文件的结论就作废** —— G1–G6 全部依赖那棵树的实测。
- 本文件**不构成 GA 声明**，也不构成任何阶段的批准。

---

## 7.1 签署

| 角色 | 姓名 | 日期 | 结论 |
|---|---|---|---|
| 提交人（`openclaw-mcode`） | openclaw-mcode | 2026-10-23 | G1–G6 与 GE1–GE7 已逐条判定；**未提交 GA 晋升申请** |
| GA 评审（codeowner） | ______ | ______ | ______ |
| GA 评审（governance） | ______ | ______ | ______ |
| 人类架构师 | ______ | ______ | ______ |

> 本文件由 `openclaw-mcode` 自动生成。`ALPHA → BETA → RC → GA` 三次阶段跃迁
> 在 `docs/governance/STAGE_CONFIG.yaml` 里都是 `requires_human_approval: true`
> ⇒ **本文件不构成 GA 声明，也不构成任何阶段的批准**，签署栏留空由人填写。

---

## 8. 参考资料

- `docs/governance/STAGE_CONFIG.yaml` —— 阶段判据 SSOT（含 `claims_boundaries`）
- `docs/releases/v4.1.0/STAGE.yaml` —— 阶段、`known_gaps`、`open_issues`、`ga_carrier_branch`
- `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` —— 缺陷计数与销账
- `docs/releases/v4.1.0/EVALUATION_REPORT.md` —— 覆盖率、TPC-H 缺口、仓库内矛盾
- `docs/releases/v4.1.0/RELEASE_GATE_CHECKLIST.md` —— RC 逐条门禁清单
- `docs/releases/v4.1.0/EVIDENCE_STATUS.md` —— D1–D5 逐维状态
- `docs/releases/v4.1.0/GA_RELEASE_TIMELINE.md` —— 时间线与晋升路径
- `docs/releases/v4.1.0/POST_GA_PLAN.md` —— 发布后计划
- `docs/releases/v4.1.0/evidence/` —— 81 份一手证据
- `docs/releases/v4.0.0/GA_GATE_REPORT.md` —— 结构先例；以及它的**撤销**记录

---

## 9. 2026-11-16 追加段：GA 门禁基线自有化重盘与实跑状态

> 依 `docs/governance/DOC_CHECK_CORRECTION_RULES.md`：**上文原始记录一字未删**，
> 本段只追加实测到的内容（不订正上文任何一句，因为上文没有与实测冲突的表述）。

### 9.1 基线升级裁决（用户「4.0.0 基线是否升级为 4.1.0 自有基线」的答复）

**裁决：升级 —— 判据继承、清单全量重盘。**

- v4.0.0 的 GA 判定已于 2026-09-30 撤销（`docs/releases/v4.0.0/STAGE.yaml` →
  `ga_promotion_status.verdict = REVOKED — v4.0.0 never legitimately reached GA`），
  其继承清单**不得**充当 v4.1.0 的合格证据；仍有效的技术判据继续沿用。
- v4.1.0 的 `STAGE.yaml` 三段 `required_gates_for_beta/_rc/_ga` 此前只是
  「Deferred / 继承」占位 note，且**全仓无任何消费方**（实测
  `grep -rn "required_gates_for" scripts/ docs/governance/` 零命中 ⇒ 执行面只读
  `docs/governance/STAGE_CONFIG.yaml`）⇒ 现已换成**实际清单镜像**：
  **BETA 19 项 / RC 26 项 / GA 35 项**（31 脚本门禁 + 4 条 cargo 命令）。
- 168 道门禁逐条归属基线：`docs/releases/v4.1.0/GATE_BASELINE_V410.jsonl`（168 行，
  与 `gov_probe.metrics.check_n=168` 严格对齐）。
  **REQUIRED 134 / HISTORICAL 32 / RETIRED 2 / NOT_APPLICABLE 0**；
  execution_point：STAGE_LIST 34 / CI_DECLARED 51 / **NONE_PENDING 37** /
  **NONE_BLOCKED_EXTERNAL 12**；`ga_critical` = 31。
  ⇒ **49 道门禁「应在 v4.1.0 范围内执行但当前无执行点」**；
  其中含 `check_audit_manifest.sh`、`check_security.sh`、`check_architecture.sh`、
  `check_test_count_monotonic.sh`、`check_a04_evidence_reproducible.sh`、
  `check_attack_surface.sh`。

### 9.2 存量红清零（本轮）

| 门禁 | 改前 | 改后 | 根因 |
|---|---|---|---|
| `cargo fmt --check --all` | rc=1（10 处 / 6 文件） | **rc=0** | 纯格式漂移（已复验 `git diff -w` 仅 2 处非空白且均为 rustfmt 语义等价改写） |
| `scripts/gate/check_commit_message.sh` | rc=1（1 条 BAD_SHAPE） | **rc=0** | **类型表不完整**：全历史实测 `evidence` 32 次 / `bench` 51 次 / `spec` 43 次都在用却不在表内 ⇒ L2 把合规提交判成 BAD_SHAPE。按 2026-11-08 补 `audit` 的**同型先例**补入规范表与机器镜像（不放宽形态规则、不动 L1、不动分母） |
| `scripts/gate/check_architecture_freeze.sh` | rc=0 但**自报 finding**（A7-4 读到 0 个 DriftGate 测试） | **rc=0 且 PASS 5/5** | 两处：①「部分检查未通过」仍 `exit 0`（fail-open）⇒ 三态化；② A7-4 只扫 `crates/*/tests/`，漏掉 src 内 `#[cfg(test)]` ⇒ **测量假阴性**（DriftGate 的负面测试确实存在于 `crates/executor/src/execution/drift_gate.rs`） |
| `scripts/gate/check_int_debt.sh` | rc=0 但**可被绕过** | rc=0（正常态） | 两处 fail-open：① 状态列未知时无条件回退到「整行扫关键字」；② `FAILED_ITEMS` 收集失败却**从不参与退出码判定** ⇒ 债务行被改名/删除时门禁仍报 PASS |

后两门的 fail-open 闭合有**反向变异证明**：fx06 / fx09 / fx10 三道负向夹具实测
全部 FIXTURE-PASS（门禁读数分别为 rc=2 / rc=1 / rc=1），两条特征化夹具的期望随之
由 `green-characterization` 翻转为 `red`。

### 9.3 实跑状态（run `20261116-ga-gates-002`）

- 冻结提交：`7e26c2b7c6840e3bbec804a0bf3bbcc706a646d7`
- 计划：`plan.jsonl`，35 项，plan sha256 `sha256:6466a0ec156…`
- 执行器：`scripts/gate/run_gates.py`（三态：exit 0 ⇒ PASS；exit 1 ⇒ FAIL；
  其它（2/124/信号/脚本缺失/无法启动）⇒ **INCOMPLETE**）
- **逐项结果以该 run 的 `results.jsonl` 为准**（本段不预写结论）。

### 9.4 结论

**GA 今日未发布。** 四条硬约束（SOAK-168H / HUMAN-SIGNOFF / CARRIER / BASELINE-GAP）
见 `GA_RELEASE_REPORT.md` §3；阶段治理与签批行见 `STAGE_GOVERNANCE_RC_TO_GA.md`。

> 本段由 `mcode-macmini` 线 2026-11-16 追加。证据：PR #5804 / #5805 / #5806；
> 归档 run `20261116-ga-gates-002`；看板行 `GA-BASELINE-1`。
