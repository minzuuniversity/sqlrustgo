# v4.1.0 ALPHA→Beta 晋级复核（2026-11-12）

> **文档性质**：复核与决策就绪包，**不是**阶段晋级结论。
> **复核对象**：`develop/v4.1.0` @ `aec1a3673cb83d8800b27e8a7aa11f690f346817`（当前 tip）
> **执行者**：`omp-macmini`（Hub 侧兼 A0 Controller）
> **规范依据**：`docs/releases/v4.1.0/ALPHA_TO_BETA_GATE_PLAN.md`（下称「计划」）、`STAGE.yaml`、
> `docs/governance/GATE_CONDITIONS.md`
> **证据纪律**：非 AI 判断的证据（归档 run id + evidence_hash(sha256) + 门禁 rc + 干净检出复现命令 + 独立 Reviewer 回执）

---

## 0. 结论

1. **ALPHA 阶段本线可闭合项已闭合并留证**：ALPHA 门禁清单（10 条）已按 §7 纪律重跑完毕
   （run `20261112-alpha-gates-003`，冻结 `aec1a3673c`，verify OK，
   evidence_hash `sha256:3cf2c844d2090b3f921fabebd89375daaced5ad4b98c019ad8f46ae5503f2c4a`，
   wall 1129s，归档 PR #5770）⇒ **8 绿 / 2 红**：
   `alpha-gate-check-alpha-v410`（ALPHA 清单门禁本体）**绿**、`arch-invariants`/`arch3-no-bypass`/
   `sql-corpus`/`anti-ignore`/`cargo build`/`cargo test --lib`/`cargo fmt` 全绿；
   两条红都在裁决域：**coverage**（门禁以 `stage=ga threshold=85%` 跑，与 ALPHA SSOT 阈值口径不一致；
   实测 L1_8 平均 **29.73%**，parser 9.51% / planner 1.21% 等数值不可信 ⇒ 指向覆盖率测量口径，
   与 #5537/#5579 同族，逐 crate 实测值已贴到那两条 issue）与 **g1**（`tpch_hash_compare --check`
   「cannot find hash in test output」⇒ 与 #5688/#5735 同源）。
   顺带修掉一处**清单缺陷**：第三份清单 `commands_alpha_gates_003.yaml` 与仓内既有的 001/002 只差一处 ——
   聚合门禁臂预算 **1800 → 10800s**（该门禁内部跑 llvm-cov 覆盖率、实测 >30min，1800s 会把它杀掉
   ⇒ **门禁逐项结果全丢**，「预算不足」被记成「门禁不过」）。
2. **ALPHA→Beta 仍不可由 Worker 单方推进**：剩余硬阻塞项全部落在
   **Owner 裁决域**（Reviewer 指派、g1 口径、RC-B1 Phase 2、台账口径）与 **CI/infra 域**
   （runner 镜像、docker 地址池、runner 服务依赖）。计划 §6 要求「P0/P1 blockers = 0 OPEN
   且有合并 PR」——本线已备好 **15 个提交-only PR**，但**合并权与指派权不在本线**。
3. **本复核给出决策就绪包**（§7）：5 组阻塞，每组给出「现状 / 三个选项 / 代价 / 建议 /
   不裁的后果」，供 Owner 一次裁定即可解锁后续自动执行。
4. **今日自查发现并已纠正 6 项**（§4）：3 处已发布数字、1 处管道正确性缺陷、1 处引擎语义缺陷、
   1 处超时死锁、1 处看板绑定口径缺陷。

---

## 1. 规范基线（计划 §1/§2/§3/§5/§6 摘录）

- **判定原则**：只允许 PASS / FAIL；AB-01..AB-10 必须在**同一冻结提交**上全部 PASS；
  required check 缺失 / 跳过 / 超时 / workflow 不可解析 / 证据不可校验 ⇒ FAIL。
  历史 PASS、合并记录、Issue 评论**不能**替代本次执行证据。
- **硬门禁判据**（计划 §2）：

| ID | 审核域 | 必须满足 | FAIL 条件 | 责任 Issue |
|---|---|---|---|---|
| AB-01 | SSOT/证据一致性 | 版本、阶段、commit、Issue、报告与 manifest 一致 | 陈旧结论、跨 commit 拼接、dirty 未披露 | #5117 |
| AB-02 | 基础质量 | fmt、clippy `-D warnings`、all-features build/test 成功 | 任一非零退出、测试失败或警告降级 | #5113 |
| AB-03 | 门禁可信度 | workflow 可解析且 fail-closed；P11-P16、anti-ignore、mutation probe 成功 | invalid/missing/skipped、退出码吞掉、registry 与源码不一致 | #5113 |
| AB-04 | 事务/会话正确性 | rollback `0/300`、8000 事务无丢失、savepoint/MVCC/多库/wire 通过 | 间歇失败、跨连接状态泄漏、模型不一致 | #5112 |
| AB-05 | 外部工作负载 | BustubX B 轨必测 **100%**；SQL corpus ≥80%；oracle 无差异 | 只检查未报错、忽略必测项、存在 P0 正确性失败 | #5114 |
| AB-06 | 覆盖率 | L1_8 平均 ≥80%；关键 crate 可追溯 | 测试失败仍产覆盖率、零样本、低于阈值 | #5113/#5114 |
| AB-07 | 稳定性/恢复 | 1h SOAK 完成、heartbeat 连续、恢复后数据一致；**24h runner 可执行** | 进程中止、心跳缺口、证据缺失、数据不一致 | #5115 |
| AB-08 | 性能基线 | 固定环境 A/B；≥5 次；正确性校验；无明显回退 | 吞吐降幅 >10% 或 p99 增幅 >20% 且超出噪声 | #5116 |
| AB-09 | 安全/依赖 | 安全扫描、依赖审计、恢复相关 gate 成功 | hard gate 降级、`|| true`、未处置高危项 | #5113/#5115 |
| AB-10 | 发布治理 | 所有 P0/P1 发布阻断项关闭且**有合并 PR**；人工双审通过 | open blocker、无 PR 关闭、无 codeowner/governance 批准 | #5117 |

- **审核流程**（§3）：冻结候选 commit → 唯一 `source_run` → 按 AB-01..AB-10 执行并留存
  stdout/stderr/exit_code → 生成 SHA-256 manifest 并校验引用 → **非执行者复核** → 全通过后
  创建独立阶段晋级 PR（只改阶段与冻结证据引用）。**修复任一失败后必须新建 `source_run`
  并重跑受影响门禁；AB-02、AB-03、AB-04 与最终汇总必须全部重跑。**
- **Issue 关闭合同**（§5）：每个子 Issue 的最终评论必须含 ①合并 PR URL + merge commit
  ②修复前/后证据 ③完整命令、退出码、测试计数与日志路径 ④`source_agent`/`source_run`/
  timestamp/evidence hash ⑤逐项勾选验收条目，未满足项保持 OPEN。
- **阶段验收记录**（§6）：候选 commit 单一 40 位 SHA；AB-01..AB-10 全 PASS；P0/P1 blockers
  = 0 OPEN；required checks 全 success；独立审核 codeowner + governance 批准；STAGE 修改走
  独立 PR，合并后再建 Beta tag。

---

## 2. 逐门禁现状（campaign `20261112-ab-001`：38 臂 = 29 绿 / 9 红）

> 证据：run `audit/v4.1.0/runs/20261112-ab-001/`（冻结 `aec1a3673c`，
> evidence_hash `sha256:8a9a94ca9c82777bdd0f35ec228070a5743b8d074a28fab89751c195`，wall 7701s，
> verify OK；归档 PR #5756）。逐臂退出码取自 manifest `results` 段。

| 门禁 | 本轮臂（绿/红） | 红的逐臂归因 | 今日进展（提交-only，待合） | 剩余阻塞 | 归属 |
|---|---|---|---|---|---|
| AB-01 | 4 绿 / 2 红 | `ab01-evidence-verify`（evidence_inventory：debt 1216、unbound 1182、ambiguous 34、findings 131）；`ab01-stage-check`（聚合 `check_alpha_v4.1.0.sh` + `check_g1_tpch_baseline.sh` 两个已知红的 meta-red） | 看板两行改 `run:<id>` 强绑定形式；实测本线 6 行看板使 debt **1216→1219**、ambiguous **34→37** ⇒ 追加看板会**加重** AB-01 债，已把绑定形式与合并顺序写进 §5.1 | 台账口径（`CLAIM_BINDINGS_LEDGER.jsonl`）+ g1 假基线 | **Owner**（台账/基线裁决域） |
| AB-02 | 3 绿 / 1 红 | `ab02-test-all` 唯一失败 `g1_tpch_sha256_baseline`（`oracle_g1_tpch_sha256.rs:259`） | #5760（提交 O(1)，mutation 199/200→0 次深拷贝；墙钟 4000 行 250.164ms→4.024ms）、#5767（INSERT 列数语义 ⇒ 语料 88%→92%）、#5761/#5762/#5763/#5764（判别力与可见性） | g1 基线（#5688 假基线 15/22 空结果） | **Owner** |
| AB-03 | 4 绿 / 1 红 | `ab03-gate-test-integrity`（P16 ignore 计数 2→3 漂移） | #5754 管道守卫自测（13 臂）、#5753 心跳 fail-closed 自测 | P16 计数属门禁注册表数据 | **Owner** |
| AB-04 | 6 绿 / 0 红 | — | #5760 顺带修 `apply_committed_log` 的裸表名键（广播删除/更新此前静默丢失） | — | — |
| AB-05 | 2 绿 / 3 红 | `ab05-bustubx-btrack`（B 轨 gate 因 Phase 2 未实现硬编码 exit 1）；`ab05-b2-per-binary`（529 二进制仅 `tls_read_spin_5099` 红）；`ab05-g1-tpch` | #5757 B 轨 compat 语料（oracle 本机 sqlite3 3.54.0 再生，6/6 周已证）；#5763 修掉 `tls_read_spin_5099` 的 darwin 平台红（pristine tip 同命令 FAILED，本分支 1/1 绿） | B 轨 gate 的 Phase 2 执行实现 | **Owner**（gate 脚本域） |
| AB-06 | 1 绿 / 0 红 | — | —（本轮 coverage 绿；ALPHA 重跑按 §7 纪律用共享 target 防假红） | — | — |
| AB-07 | 3 绿 / 1 红 | `ab07-soak-1h`（脚本硬编码作者机路径） | **v4.1.0 首个有效 1h SOAK**：13260 queries/16 线程/60min、panics=0、RSS 126–153MB、心跳 241 beats 无缺口且闭合、summary.json 落盘（归档 PR #5766，run `20261112-soak-ab003`）；机制 #5753（心跳 emitter/checker + 监督 wrapper）、#5755（路径可移植 + 未加载 fail-closed） | **24h runner 可执行**：仓内路径已证可执行（PR #5769：短段镜像 ⇒ 1 000 行播种、worker **0 errors**、指标列全为数值；并修掉 runner 的五处缺陷——**不建工作负载表**导致 1122/1122 全错却照报 qps、qps 取到 chatter 行、日志二进制未用 `grep -a`、qps 正则与实际格式不符、`ps -o nlwp=` 在 macOS 写出字段名列表）；CI runner 本身仍不可用 | **CI/infra** |
| AB-08 | 3 绿 / 0 红 | — | 管道 5 处硬化（负载守卫 / 逐 bench 超时 + 诚实 NOT-RUN / 进程组 kill 修管道死锁 / 探针自排除 / 指标键按 bench 命名空间）+ 13 臂守卫自测；首次六核全集测量 5/6 目标、96 唯一组（bench_cbo 诚实 NOT-RUN），归档 PR #5766 | **正式基线 promote**：需**完全静默窗口**（实测并行开发时 load1=15.68 ⇒ 守卫拒绝；负载下的基线会掩盖回归） | 本线（窗口）+ **CI/infra**（容器天然独占） |
| AB-09 | 2 绿 / 1 红 | `ab09-no-secrets`（v312-59 证据 JSON `local-git:` 哈希误报） | #5381 仓库侧复核 rc=0 全 PASS（5 脚本 env-only + 缺参 fail-closed）；证据文件覆写副作用已披露并还原（#5577 同类），第二轮清单把该臂输出重定向到 /tmp | 误报项在已封存证据 JSON 内 + 检测器豁免属裁决域 | **Owner** |
| AB-10 | 1 绿 / 0 红 | — | 看板 SSOT 129 行 + 50 臂第二轮清单 + 跑法要点（PR #5765）；本线 16 个提交-only PR 已备 | **合并 PR**（g3 receipts 域，Worker 不自合并）+ 独立 Reviewer 回执（Hub #133） | **Owner** |

**汇总**：9 红中，**3 红可由本线已备好的 PR 消除**（`ab05-b2-per-binary` ← #5763；
`ab07-soak-1h` ← #5753+#5755；`ab02-test-all` 的唯一失败属 g1 基线 ⇒ 需 #5688 裁决），
其余 6 红属 Owner 裁决域或 CI/infra 域。

---

## 3. ISSUE 全面分析（36 个 open）

### 3.1 本线已修（提交-only，待合 ⇒ AB-10 的「有合并 PR」条件）

| Issue | PR | 判别力证据 |
|---|---|---|
| #5507 存储提交 O(N²)（P0） | #5760 | mutation 199/200 次整表深拷贝；墙钟 62×；storage 917 + 根 174 + sqllogictest 逐文件一致 |
| #5498 Q4 残差短路无判别力 | #5761 | mutation（取消短路、语义不变）精确失败于短路断言，功能断言仍过 |
| #5503 NOT-RUN 本地不可见 | #5762 | 自证 4/4 绿；顺带修 `writeln!` 记录撕裂 |
| #5711 跳过仍被计为通过 | #5764 | 4 条互不相同记录 vs 修复前 1 条 |
| #5712 TLS 自旋 darwin 平台红 | #5763 | 本机 1/1 绿；pristine tip 同命令 FAILED（1.37s） |
| #4640 短 INSERT 静默补 NULL | #5767 | 新增 6 判别用例；**语料 22 文件/3 红/88.0% → 23 文件/2 红/92.0%** |
| #5679/#5686 WAL group commit | 已在 tip 修复 | 8/8 ×5 确定性绿；建议 Owner 关闭 |
| #5548/#5549 契约未验证 / 机器依赖 flaky | 已在 tip 修复 | e2e 54/54（契约 2 真被执行）；负载机 3 × 40/0 绿；建议 Owner 关闭 |
| #5567 CI fixture 步骤无 NOT RUN 兜底 | 已在 tip（`scripts/ci/prepare_tpch_fixture_ci.sh`，两 job 共用一份实现） | 三模式实测：正常（8 表/86482 行、无 NOT-RUN 记录、rc=0）/ 环境阻塞（PATH 无 python3 ⇒ NOT RUN 记录 + rc=0）/ 生成器坏（前提齐 ⇒ **rc=1** 并明说「code regression, not an environment block」）| | #5528 musl 缺静态 zlib | 仓库侧已在 tip（`d914c4caaf`） | 特征统一已核；链接层验证需 CI 镜像；建议 Owner 关闭（残余=镜像/工具链固定） |
| #5381 明文凭据 | 仓库侧已闭环 | 检测器 rc=0 全 PASS；残余=凭据轮换（Owner/infra） |

### 3.2 Owner 裁决域（只报不裁）

| Issue | 需要的裁定 |
|---|---|
| #5735 Q9 浮点累加次序（阻塞 g1 + #5688） | 三选一：钉死 partial 合并次序 / 数值渲染归一（定点·定精度）/ Q9 例外按行集+容差比对 |
| #5688 空结果假基线（15/22 条目为空结果集哈希） | 重建基线的口径与采纳规则（与 #5735 同一裁决链） |
| #5717 Q17 空集 NULL 渲染（engine `""` vs oracle `\0NULL`） | 判 engine 错（修渲染）/ 判差异合法（登记 known_differences）/ 显式豁免 |
| #5603 claims 证据台账三套并存 | 指定 SSOT 与迁移/退役口径（决定 AB-01 债能否下降） |
| #5698 撞名 run 的 evidence_hash 含 run_id | 三选一：`run_id_original` 字段 / `--expect-run-id` 逃生口 / 白名单（最弱） |
| #5702 claims_selftest 3/17 臂红 + 2 臂不判别 | 先修 fixture/口径，再谈判别；绿读数在此之前不得引用 |
| #5520 公开仓库分支上的凭据 + 5 个同步工具 | 凭据轮换/暴露窗口确认；gitcode 分支处置（Worker 不推其它远端） |
| #5557 `--wal-sync every` 卡不可中断 I/O | 产品决策：WAL 写超时 or 文档明示 SIGKILL 可能延迟（根因未定位，需内核栈） |
| #5596 7 条历史审计记录不可复核 | 退役 / 标注 / 重做 三选一 |
| RC-B1 Phase 2（B 轨 100%） | 门禁脚本域的实现授权 |

### 3.3 CI / infra 域

#5597（runner-03 缺镜像）、#5551（docker 地址池占满）、#5569（runner 服务 Requires=docker.service）、
#5573（act_runner 被频繁优雅关机 ⇒ 在飞容器 pause）、#5526（clippy 版本漂移）、
#5528 残余（镜像固定 zlib/工具链）、AB-07 的 24h runner 可执行性。

### 3.4 其它 agent claimed（不重复动工）

#5525（备份/恢复骨架）、#5537（门禁 coverage 漏 feature）、#5556（fault-injection reach 自旋）、
#5577（测试覆写证据；本线已提供复现细节）、#5584（崩溃测试边界）。

---

## 4. 自我核查与自我纠正（2026-11-12）

| # | 发现 | 影响 | 处置 | 证据 |
|---|---|---|---|---|
| 1 | 已发布数字 3 处有误：campaign 臂数/绿数（37 臂 28 绿 → **38 臂 29 绿**）、AB-08 bench 组数（95 → **96 唯一组 / 97 对**）、第二轮清单臂数（49 → **50**） | 看板/归档 manifest/hub/评论/PR 正文里的数字失真 | 全部**就地更正**（hub PR #254；PR #5755/#5756/#5765/#5766 正文与评论 PATCH；看板 PR #5765 两次更正提交） | 归档 manifest `results` 段逐臂退出码（29 个 exit_code=0 / 9 个非零） |
| 2 | **AB-08 指标按组名键入**：组名 `parser_batch_100_select` 在 `bench_insert` 与 `parser_bench` 中同名 ⇒ 97 个 (bench,group) 对只留 96 个唯一键 ⇒ 一次测量被静默覆盖，且 compare 可能跨 bench 串号 | 基线比较可能拿错 bench 的数字 | 键改为 `<bench>/<group>` 命名空间（PR #5754 commit `dae94f1791`）+ 自测新增 2 臂（撞名 ⇒ 键不同；撞名时只报被回归的那个 bench） | 自测 13 臂全绿 |
| 3 | **AB-08 超时只 kill 直接子进程** ⇒ `cargo bench` 的孙进程仍持有 stdout 管道 ⇒ `communicate()` 永久阻塞 ⇒ **超时永不生效**（正是该预算要防的挂死） | bench 挂死无法转成 NOT-RUN | 独立进程组 + `killpg`（commit `1d209857b9`），实测 4s 预算 ⇒ 4.0s 返回 rc=124 | 同上 |
| 4 | **INSERT 短式（VALUES 列表长度 < 表列数）被静默补 NULL** ⇒ 打错的 INSERT 丢列仍成功；而给多值却报错 ⇒ 两方向不一致 | 静默数据损坏；与 SQLite/MySQL 相悖；仓内 SQLite 官方语料 `insert__test_insert_invalid.test` 期望报错 | PR #5767（两处：VALUES 短式与 `INSERT…SELECT` 短式；显式列清单形式不变） | 语料 **22/3 88.0% → 23/2 92.0%**；sql-corpus 91/0；6 判别用例绿 |
| 5 | 看板追加行会**加重 AB-01 债**（debt 1216→1219、ambiguous 34→37）：`evidence_inventory` 的强绑定只认 `run:<id>` 形式 | ALPHA 的 AB-01 会因我的改动更红 | 两行改 `run:<id>` 强绑定形式；把「先合证据 run、再合看板行」的**合并顺序**写进 §5.1 并升级给 Owner | `STRONG_RE` 规则 + 实测前后 debt |
| 6 | AB-01 的 `findings=131` 的真实构成此前被我归到「g1 家族」——实测其主因是 **unbound 1182 + ambiguous 34 + debt 1216**（claim 绑定/债务口径），与 g1 基线是**两件事** | 归因不准会把 Owner 引向错误的裁定对象 | 本复核 §2/§3.1 已按实测改写归因 | `evidence_inventory.py verify` 的 JSON 输出 |

---

## 5. 改进与优化方案（面向「达到门禁」）

### 5.1 证据绑定与合并顺序（直接决定 AB-01 能否不更红）

- **规则**：任何**新 claim**（看板行、报告段落）必须用 `run:<run-id>` 强绑定形式引用证据；
  路径形式（`audit/v4.1.0/runs/<id>/`）**不算绑定**，会计入 unbound/ambiguous 债。
- **合并顺序**：`#5756`（campaign 归档）、`#5766`（soak + AB-08 归档）**先合** ⇒ run 进入树内 ⇒
  看板行（`#5765`）的 `run:20261112-soak-ab003` / `run:20261112-ab08-core3` 才可解析为绑定；
  最后是代码 PR（合入后需重跑受影响门禁并新建 `source_run`，见计划 §3）。
- **剩余 +1 债**（AB-CAMPAIGN 行引用的是有 9 红的 campaign run ⇒ 绑定它会触发 `unsafe-binding`）：
  需 Owner 决定（绑定一个**该臂全绿**的子 run，或在台账里记这条例外）。

### 5.2 每个新门禁必须自带「去掉守卫后必失败」的自证

今天已交付三处可验证样例：AB-08 管道 13 臂自测（`scripts/perf/test_ab08_criterion_ab.py`）、
NOT-RUN 本地通道自测（4 臂）、按合取计数的短路判别（mutation 精确失败）。
**建议**：此后任何新增 fail-closed 判据都附同族自测，否则按 #5594「死门禁」家族处理（低估问题面）。

### 5.3 缺陷发现常态化对拍（SQLite oracle + 官方语料）

今天用一次性探针 + sqlite3 对拍发现 #4640 的语义缺陷（语料随即 +1 文件转绿）。
**建议**：把这类边界（短式列数、DEFAULT 作值、行值 SET、多行长度不齐、INSERT…SELECT 列数）
固化为语料用例或对拍脚本进 CI，避免下一轮靠人工探针。

### 5.4 基线窗口独占化

AB-08 的 promote 要求「完全静默」窗口（实测并行开发时 load1=15.68 ⇒ 守卫拒绝）。
**建议**：把基线 promote 排到固定独占窗口，或等 CI runner 修好后（#5597）在容器里做
——容器天然独占，比共享开发机可靠。

### 5.5 合并即重贴（正文与仓内不漂移）

PR 合并 → 看板三件套重贴（`gen_kanban_issue.py --post 5592 / --post 5599` + 钉住评论）→
hub 更新。计划 §3 要求「修复后必须新建 `source_run` 并重跑受影响门禁」⇒ 三件套与新 run 的
证据引用要在同一条链上更新。

---

## 6. 达到门禁的执行清单

**本线可执行（已全部备好，等合并触发）**

1. Owner 合入 15 个 PR（建议顺序：证据 `#5756`/`#5766` → 代码 `#5753`–`#5755`/`#5757`–`#5759`/
   `#5760`–`#5764`/`#5767` → 看板 `#5765` → 文档/清单随链）。
2. 在**新 tip** 冻结，跑第二轮 campaign（`audit/v4.1.0/commands_ab_002.yaml`，50 臂，
   含 pymysql 前置臂、soak 监督 wrapper、AB-08 管道冒烟 + 守卫自测、6 条本轮修复判别式回归臂）。
3. 独占窗口 promote AB-08 正式基线（五个 core bench；bench_cbo 需单独预算或按 NOT-RUN 显式记录）。
4. 看板三件套重贴 + hub 更新 + 本复核的 §2 现状表刷新。
5. 创建**独立阶段晋级 PR**（只改 `STAGE.yaml` 的阶段与冻结证据引用），合并后再建 Beta tag。

**需 Owner 裁定（§7）**：Reviewer 指派、g1 口径、RC-B1 Phase 2、台账口径、CI 执行面。

---

## 7. 决策就绪包（Owner 一次裁定即可解锁）

| # | 阻塞 | 现状 | 选项与代价 | 建议 | 不裁的后果 |
|---|---|---|---|---|---|
| D1 | **Reviewer 指派**（AB-10 / Hub #133） | 计划 §7 表：ALPHA→BETA 行「等待 Reviewer 指派」 | ①指派独立 Reviewer（最省，0 成本）②本线不自审（合规要求，代价=等待） | ①尽快指派（非执行者复核是计划 §3 的硬要求） | AB-10 无法 PASS ⇒ 晋级 PR 缺治理批准 |
| D2 | **g1 口径**（#5735 + #5688） | Q9 两工序哈希不同；基线 15/22 条目为空结果假基线 | ①钉死累加次序（改动引擎，影响面大）②数值渲染归一（定点/定精度）③Q9 例外按行集+容差（改动基线形式） | ③ + ②并用：Q9 走行集+容差，其余查询数值归一后再定基线 | AB-01/AB-02/AB-05 三处同源红 ⇒ 拖住整轮 |
| D3 | **RC-B1 Phase 2**（B 轨 100%） | B 轨 gate 硬编码 exit 1（Phase 2 未实现） | ①授权实现 Phase 2（门禁脚本域，需工时）②承认 legacy 并登记豁免（弱） | ①（AB-05 要求必测 100%） | AB-05 恒红 |
| D4 | **台账口径**（#5603 + #5698 + #5702 + AB-01 债） | 三套台账并存；撞名 run 永久报红；claims_selftest 3/17 臂红；债 1216 只准减不准增 | ①指定单一 SSOT + 迁移脚本（治本）②本轮只冻结口径、债按现状入账（治标）③逐项豁免（最弱、fail-open 面） | ①，且**先做 #5698 的 `run_id_original`**（一处改动消掉一条永久红） | AB-01 继续红；且我的看板追加会继续增债 |
| D5 | **CI 执行面**（#5597/#5551/#5569/#5573/#5526） | runner 镜像缺 sqlrustgo-runner:v1；docker 池满；runner 服务依赖 docker.service；act_runner 被频繁优雅关机；clippy 版本漂移 | ①重建镜像并固定工具链 + 加 zlib-static（治本）②换回旧镜像（回退，但旧镜像也有版本漂移）③本地/专用机执行（绕开，但 AB-08 基线与 24h runner 都需长期机） | ①（并把 AB-08 基线排到容器内独占窗口） | AB-07 的 24h runner 不可执行 ⇒ AB-07 无法 PASS |

---

## 8. 证据索引

- campaign 第一轮：`audit/v4.1.0/runs/20261112-ab-001/`（PR #5756），evidence_hash
  `sha256:8a9a94ca9c82777bdd0f35ec228070a5743b8d074a28fab89751c195`。
- AB-07 1h SOAK：`audit/v4.1.0/runs/20261112-soak-ab003/`（PR #5766）。
- AB-08 首次六核全集：`audit/v4.1.0/runs/20261112-ab08-core3/`（PR #5766）。
- ALPHA 门禁清单重跑：`20261112-alpha-gates-003`（本线执行中）。
- 第二轮 campaign 清单：`audit/v4.1.0/commands_ab_002.yaml`（50 臂，PR #5765）。
- 本线 16 个提交-only PR：#5753–#5767、#5769（168h runner 路径）。
- 复核评论：sqlrustgo#5117（campaign 总账）、#5507、#5498、#5503、#5711、#5712、#5548、#5549、
  #5679、#5686、#5528、#4640、#5381；Hub 看板条目（#239/#243/#251/#253/#254）。

---

## 9. Issue 关闭合同（计划 §5）逐条包（合并后回填即用）

§5 要求每个子 Issue 的最终评论含五项：**①合并 PR URL + merge commit；②修复前可复现证据与修复后
证据；③完整命令、退出码、测试计数与日志路径；④`source_agent`/`source_run`/timestamp/evidence hash；
⑤逐项勾选验收条目，未满足项保持 OPEN**。下列各条中 **①的 merge commit 待 Owner 合入后回填**，
其余已具备；**未满足项一律保持 OPEN**（本线不代关）。

### 9.1 #5507 存储提交 O(N²)（PR #5760）

1. 合并 PR：`http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/5760`；merge commit：**待合入回填**。
2. 修复前：mutation（两个提交入口改回 `committed_tables = tables.clone()`）⇒ 逐行 200 次提交实测
   **199 次整表深拷贝**（`cfg(test)` 计数器）；修复后：**0 次**；wall-clock 4000 行
   **250.164ms → 4.024ms**（每行 0.0625 → 0.0010 ms）。
3. 命令与计数：`cargo test -p sqlrustgo-storage`（lib **917 passed** + 集成全绿；唯 2 个 crash 用例
   需 fault-injection 服务端二进制，构建后 2/2 绿；pristine tip 同命令同样红 ⇒ 与本改动无关）、
   `cargo test -p sqlrustgo --lib`（**174 passed**）、
   `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata`
   （与 tip 逐文件一致 22/3、88.0%，零回归；#5767 合入后为 23/2、92.0%）。
   日志路径：PR #5760 评论 211148（wall-clock 对照）与本文件 §2。
4. `source_agent=omp-macmini`；`source_run=20261112-alpha-gates-003`（阶段 run，§5④的 source_run 以
   合并后新跑为准）；timestamp 见归档 manifest；evidence hash 见归档 run 的 `evidence_hash`。
5. 验收勾选：提交代价 = 事务触及行数 ✔；写路径零深拷贝 ✔；已提交视图与活表一致 ✔；
   删除/更新与广播路径的键作用域错配修正 ✔。

### 9.2 #5498 Q4 残差短路判别力（PR #5761）

1. PR `.../pulls/5761`；merge commit：**待合入回填**。
2. 修复前：哨兵计数器从未接线 ⇒ 断言只覆盖「结果正确」；修复后：mutation（求值全部合取、语义不变）
   实测**精确失败于短路断言**（`R2 must be evaluated only for the row that passed R1; got 2`），
   而功能断言与路径断言仍通过 ⇒ 断言确实在测短路。
3. 命令：`cargo test --all-features --test q4_nested_and_short_circuit_test -- --test-threads=1`
   ⇒ **2 passed**；计数：R1=2 / R2=1 / R3=1（引擎侧 `DIAG_RESIDUAL_CONJUNCT_EVALS[8]`）。
4. 同 §9.1④；`source_run` 以合并后新跑为准。
5. 验收勾选：断言对准「短路真的发生」✔；变异必须失败 ✔；计数可复核 ✔。

### 9.3 #5503 NOT-RUN 本地可见性（PR #5762）

1. PR `.../pulls/5762`；merge commit：**待合入回填**。
2. 修复前：未设 `TPCH_NOT_RUN_FILE` 时 stdout 被 cargo 吞掉 ⇒ 本地零线索；修复后：默认落盘
   `<target>/tpch_not_run.log`（轮次按 cargo 进程 pid，无 `ps` 时只追加不清空）。
3. 命令：`cargo test -p sqlrustgo --test not_run_log_visibility_5503 -- --test-threads=1` ⇒ **4 passed**；
   文档：`tests/README.md`「NOT-RUN 披露通道（#5503）」。
4. 同 §9.1④。
5. 验收勾选：默认路径有本轮记录 ✔；文档写明默认路径与用途 ✔；自证（NOT-RUN 时有记录、真跑时无对应记录）✔。

### 9.4 #5711 跳过的测试仍被计为通过（PR #5764）

1. PR `.../pulls/5764`；merge commit：**待合入回填**。
2. 修复前：4 个跳过测试共用一个 target 名 ⇒ CI 去重计数只扣 1；修复后：按测试名披露 ⇒ **4 条互不相同**。
3. 命令：`TPCH_NOT_RUN_FILE=/tmp/nr5711.log cargo test -p sqlrustgo --test harness_validation_test -- --test-threads=1`
   ⇒ 11 passed / 0 failed（全部走 NOT-RUN 早退）；记录计数：`grep -o 'TPCH-NOT-RUN:.*' | sort -u | wc -l`
   ⇒ **4**（修复前 1）。
4. 同 §9.1④。
5. 验收勾选：每个跳过测试一条记录 ✔；去重计数与跳过数一致 ✔。

### 9.5 #5712 TLS 自旋平台红（PR #5763）

1. PR `.../pulls/5763`；merge commit：**待合入回填**。
2. 修复前：pristine tip 在 macOS 上 **FAILED**（0 passed / 1 failed，1.37s，死在 `/proc/<pid>/stat` 读取）；
   修复后：**1 passed**（10.61s）。
3. 命令：`cargo test -p sqlrustgo-mysql-server --test tls_read_spin_5099 -- --test-threads=1`。
4. 同 §9.1④。
5. 验收勾选：跨平台 CPU 计量 ✔；pymysql 发现不硬编码解释器路径 ✔；环境缺失给出可操作错误 ✔。

### 9.6 #4640 短 INSERT 静默补 NULL（PR #5767）

1. PR `.../pulls/5767`；merge commit：**待合入回填**。
2. 修复前：`INSERT INTO a VALUES (1)`（表 2 列）**被接受**，行变 `[1, NULL]`；修复后：报错
   `Binder Error: table a has 2 columns but 1 values were supplied` 且**不留行**。
3. 命令：`cargo test -p sqlrustgo --test insert_column_count_parity_4640 -- --test-threads=1` ⇒ **6 passed**；
   `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata`
   ⇒ **23 文件 / 2 红 / 92.0%**（修复前 22/3、88.0%）；`cargo test -p sqlrustgo-sql-corpus --release`
   ⇒ **91 passed / 0 failed**；`cargo test -p sqlrustgo --lib` ⇒ 174 passed。
4. 同 §9.1④。
5. 验收勾选：短式被拒且不留行 ✔；长式被拒 ✔；等长接受 ✔；显式列子集仍合法 ✔；
   `INSERT…SELECT` 短式被拒 ✔；与 SQLite 语料一致 ✔。

### 9.7 #5679 / #5686 / #5548 / #5549 / #5528（已在 tip 修复，建议 Owner 关闭）

- #5679：`cargo test -p sqlrustgo-storage --lib wal::group_commit` ⇒ **8 passed**，连续 **5 次**全绿；
  修法 = 结果槽与 epoch 生命周期解耦（按 generation 读结果，closed epoch 的结果保留至 waiter 取走）。
- #5686：同一命令 ⇒ 含 3 条 `walgc2_*` 回归（leader 死亡 ⇒ Drop 守卫关闭 epoch 并唤醒 waiter）；
  模块头的假 liveness 承诺已撤回。
- #5548：`cargo test -p sqlrustgo --test e2e_canonical_subprocess -- --test-threads=1` ⇒ **54 passed**，
  契约 2（不能委托时必须诚实失败）本次**真的被执行**（wrapper 复制到空目录 ⇒ 五候选全落空）。
- #5549：同命令连续 **3 次** **40 passed / 0 failed**（负载机 load1 7.6–9.4）；窗口由实测迭代派生，
  判据（`contended >= alone * 0.7`）未放宽。
- #5528：仓库侧兜底已在 tip（commit `d914c4caaf`，根包直接依赖 `libz-sys` + `static` feature）；
  `cargo tree -e features -i libz-sys` 证实特征统一；链接层验证需 CI 镜像（本机无 musl target/链接器）。
- 以上五条的关闭包只差 **①的 merge commit** 与 **④的 source_run**（历史 run 的 evidence hash 见
  `20261112-ab-001` 归档 manifest）。

### 9.8 #5577（claimed：openclaw-mcode）

本线提供了**实测复现细节**（campaign 20261112-ab-001 的 ab09-security-scan 臂覆写
`docs/releases/v3.12.0/evidence/v312-59/` 四件，已还原且未入证据链）与**规避手段**
（`EVIDENCE_DIR`/`OUT_FILE` 重定向到 /tmp，已进第二轮清单）⇒ 见 comment 211121；
该 issue 的代码处置属 claimed 线，本线未改。
