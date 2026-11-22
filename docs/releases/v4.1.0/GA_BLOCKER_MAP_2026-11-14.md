# v4.1.0 GA 阻断台账（草案，2026-11-14）

> **性质**：执行者整理的**提案**，不是裁定。分类需 Owner 与独立 Reviewer 逐条确认。
> **当前结论**：Stage = `ALPHA`（`STAGE.yaml` SSOT），`ga_declared: false`；
> **当前不可执行 Alpha→Beta / RC / GA 晋级**。本文件只回答“要补什么证据、按什么顺序”。
> **输入**：`STAGE.yaml`、`STAGE_CONFIG.yaml`（168 门禁 / 阶段 required_gates）、
> `KANBAN_v4.1.0_ALPHA`（135 行）、Gitea open `priority/p0`（31 条，2026-11-14 快照）、
> 外部评审（chatGPT-web `web-session-thinking`，2026-11-14；deepseek-web 本轮 HTTP 500）。
> **证据纪律**：任何状态都必须能指回归档 run + `evidence_hash`；本文件标 `UNMEASURED`
> 的项不得在晋级时被当作 PASS。

---

## 1. 为什么当前不能晋级（事实，不是判断）

| # | 硬约束 | 现状 | 来源 |
|---|---|---|---|
| C1 | 阶段必须逐级 ALPHA→BETA→RC→GA | 当前 ALPHA；BETA 门禁未逐项取证 | `STAGE.yaml current_stage`；`ALPHA_TO_BETA_GATE_PLAN.md` AB-01..AB-10 |
| C2 | GA 只能在 `main` / `release/v*` / `hotfix/v*` 判定 | `release/v4.1.0` **不存在** | `STAGE_CONFIG.yaml stages.GA.branch_pattern`；#5436 |
| C3 | CI 必须提供 lint/build/test/full-gate/anti-fab 证据 | **2026-11-14 恢复中**：runner-02/03/04 online，job 真执行；run 12569 的 `lint-build`/`mutation`/`test-soak-detach` 已 success（1.99 新 lint 由 #5785 修复），`test`/`test-crates`/`postcheck` 被仓库级 concurrency 取消（非失败）。**尚无一次完整 run 全绿**；#5402 的 postcheck 仍未拿到 success。另：run 12581 的 `test-crates` 在 `cargo build` 阶段被 crates.io 网络拖住（`spurious network error: transfer too slow ... 120s`，`cache: no cached registry`），属 #5451/#5533 的外部依赖面 | #5597 / #5402 / #5451 |
| C4 | 独立 Reviewer 回执 | Hub `ai-project-hub` #133 未回；不许自签 | `STAGE.yaml merge_policy`；ADR-014 |
| C5 | 覆盖率 ≥ 85%（GA） | 最近单 crate 读数：parser 86.20%、mysql-server 87.89%、storage 87.70%；**workspace 全局 85% 未在冻结快照上重测** | COV-01/02 归档；#5579 |
| C6 | 168h SOAK | Owner 2026-11-13 裁决「推迟」= **deferral，不是 PASS** | `STAGE.yaml` V400-09；看板 ING-1 |
| C7 | 强制门禁必须 PASS 或具合法例外 | `check_audit_manifest.sh` rc=1（9 条红，见 §4；canonical 已修 10→9）；G1 基线哈希 2026-11-14 实测红（`7439ce80…` vs `b8854271…`，但原始 LFS 夹具不可复现 ⇒ 判 INCOMPLETE，见 `G1_PROVENANCE_2026-11-14.md`）；IDX-2 已提交修复 PR #5789（待复核） | 本会话实测 |
| C8 | merge_policy | GA: 3 reviewers + codeowner + governance + human-architect + release-manager | `STAGE_CONFIG.yaml stages.GA.merge_policy`；#5452 |

> 结论：C1–C8 中任意一条未满足都不足以发表 GA 结论。当前至少 C2/C3/C4/C5(workspace)/C6/C7 未满足。

---

## 2. P0 分类（提案，需 Owner 裁定）

分类口径：
- **阻断 GA**：影响发布结论的合法性/审计可信性/发布路径；不解决就不能宣布 GA。
- **阻断 RC**：影响 RC 候选的可用性/正确性/CI 证据；不解决就不能切 `v4.1.0-rc1`。
- **可接受边界（候选）**：仅在 Owner 明确接受、阶段规则允许、影响面可证、release notes 如实披露时成立；**当前一律未获豁免**。

| Issue | 主题 | 提案分类 | 当前证据 / 缺口 | 关联门禁或阶段项 | 下一步 |
|---|---|---|---|---|---|
| #5603 | claims 证据台账三套并存 | **阻断 GA** | 四份台账文件并存（ls 实测）；无单一权威来源 | 审计可信性；#5596 | 定一份 SSOT，其余标 retired 并留哈希 |
| #5596 | 7 条历史审计不可复核 | **阻断 GA** | 5 hollow 无命令行 / 1 未封板 / 1 无清单 | `check_audit_manifest.sh` | 逐条：重跑 / 显式排除 / 退役（Owner 裁） |
| #5235 | IAIA-410 审计 Master | **阻断 GA** | Master 未收口 | GA 审计规则 | 子项 A01–A12 逐项关闭或裁定边界 |
| #5234 | A12 最终审计报告/审批衔接 | **阻断 GA** | 无最终报告 | GA required_gates + 审计 | 报告 + Owner 审批记录 |
| #5233 | A11 Independent Judge 复核 | **阻断 GA** | 无独立裁决 | ADR-014 / Reviewer | 指派独立 Judge，出回执 |
| #5228 | A06 事务/WAL/恢复审计 | **阻断 GA** | 审计未完成 | GA 审计 | 按 IAIA-410 清单执行 |
| #5227 | A05 索引/BufferPool/持久化审计 | **阻断 GA** | 审计未完成；IDX-2 相关 | GA 审计 + IDX-2 | 先闭 IDX-2 再审计 |
| #5226 | A04 SQL 语义/协议差分审计 | **阻断 GA** | 审计未完成；0xfd 属协议面 | GA 审计 | 审计覆盖 0xfd 修复 |
| #5223 | A01 冻结基线/manifest 契约 | **阻断 GA** | manifest 契约本身有红项 | `check_audit_manifest.sh` | 与 #5596/#5603 合并处置 |
| #5204 | AOCI-CODE 索引阻塞 | **阻断 GA** | 索引内嵌机器绝对路径；治理在 HEAD 阻塞 | 治理工具链 | 重建索引或裁为边界（需 Owner） |
| #5452 | GA merge_policy 与分支保护不一致 | **阻断 GA** | STAGE_CONFIG GA 要求 3 reviewers + 4 类审批；分支保护未对齐 | `stages.GA.merge_policy` | 对齐分支保护或 Owner 书面例外 |
| #5454 | release-please 链路不存在 / PR 标题锁死 | **阻断 GA** | 仓库无 release-please；提交类型与实际流程冲突 | 发布流程 | 落地或正式废止并改规范 |
| #5520 | 5 远端同步工具 + 明文凭据残留 | **阻断 GA** | 仓库侧已收敛；gitcode 公开分支仍可读旧明文 | 安全/供应链 | 服务端轮换 + 公开分支清理证据 |
| #5117 | Alpha→Beta 总控 | **阻断 RC**（前置） | AB-01..AB-10 未逐项取证 | `ALPHA_TO_BETA_GATE_PLAN.md` | 先完成 Beta 晋级，不跳级 |
| #5597 | runner-03 缺镜像，8 job 未执行 | **阻断 RC** | 0 job 执行 | CI required | 修镜像/换 runner，留 CI run id |
| #5573 | act_runner 每 6–9 分钟优雅关机 | **阻断 RC** | 在飞 job 被 pause | CI 稳定性 | 修 runner 生命周期 |
| #5569 | gmp runner Requires=docker.service | **阻断 RC** | 重启依赖导致 job 断 | CI 稳定性 | 解耦服务依赖 |
| #5551 | docker 地址池耗尽 | **阻断 RC** | Start image 全灭 | CI | 扩池/清理 |
| #5528 | Alpine/musl 缺静态 zlib | **阻断 RC** | 链接测试二进制全红 | CI | 补 zlib 或换镜像 |
| #5402 | postcheck 600 run 从未成功 | **阻断 RC** | 6 个硬阻塞 meta-gate 形同虚设 | postcheck | 恢复执行并留 rc |
| #5388 | fail-fast 掩盖 382 个测试目标 | **阻断 RC** | 未执行目标未归因 | 全量测试 | 全量跑 + 分流 |
| #5363 | act_runner 未注册 | **阻断 RC** | 50/50 queued | CI | 注册 runner |
| #5364 | evidence-graph 短 hash panic | **阻断 RC** | panic 复现 | evidence-graph | 修 ingest |
| #5366 | evidence-graph 结构上无法 PASS | **阻断 RC** | SHA 未回退/边早于节点/缺 VERIFIED_BY | evidence-graph | 修图模型 |
| #5368 | evidence-graph 解析不了自己的 PASS | **阻断 RC** | grep 与 pretty JSON 不匹配 | evidence-graph | 修解析 |
| #5584 | 崩溃测试没有在声称边界崩溃 | **阻断 RC** | 另一线 `state/claimed`；看板 ENG-13 仍阻塞 | G8 crash matrix | 等认领线交付 + 本线复证 |
| IDX-2 | 页式存储建索引后静默丢行 | **阻断 RC**（数据正确性） | 看板 IDX-2 仍登记为产品缺陷 | 索引/存储正确性 | 修 row-id 寻址；不接受为边界 |
| #5428 | quick_query 依赖 `/tmp/tpch-sf1` | **阻断 RC** | 干净环境必失败 | G1 / 自包含测试 | 改为仓内 fixture 或显式 NOT-RUN |
| G1 wire | TPC-H wire 侧 Q9/Q18/Q19/资源限制 + **基线哈希** | **阻断 RC**（若 G1 属 RC 必需） | 文件后端 22/22；2026-11-14 实测：无夹具 ⇒ NOT-RUN；SF=0.01 ⇒ `profile_mismatch`；SF=0.001 生成夹具 ⇒ hash mismatch（actual `7439ce80…` vs baseline `b8854271…`）；14 个 seed 无一命中。**关键**：v3.9.0 原始夹具是 LFS 指针，本机/252 LFS 端点不可用 ⇒ 无法复现原始 profile；当前生成器在 v3.9.0 后改过两次。详见 `G1_PROVENANCE_2026-11-14.md` | `check_g1_tpch_baseline.sh` / `tests/tpch_hashes_v380.json` | 专家建议：保持 **INCOMPLETE**；先尝试恢复 v3.9.0 LFS 夹具（选项 A），否则做逐 query provenance 后建立**新基线**并明确标注不是历史连续（选项 B）；**禁止**直接 bump/换 profile |
| #5557 | server 大量写入后不可中断 I/O | **阻断 RC** | 进程状态 U；SIGTERM 无效 | 运维/崩溃恢复 | 定位 fsync/IO 路径 |
| #5362 | clippy -D warnings 失败 | **阻断 RC（临）** | PR #5785 已修，待合并验收 | RC `cargo clippy` | 合并后关闭 |
| #5412 / #5507 | 逐行 INSERT O(N²) / 提交快照 | **候选边界（性能）** | #5507 有修复线；性能门禁 G11/G12 为 optional | GA optional_gates | 若 GA 性能目标要求则升阻断；否则 release notes 披露 |
| V400-09 | 168h SOAK | **候选边界（需 Owner 明确适用于 GA）** | Owner 2026-11-13「推迟」；非 PASS | GA optional_gates / G7/G13 | 取得 GA 适用裁定或有效 SOAK |
| SEC-1 | 12 条假覆盖契约 | **候选边界（需 Owner 裁定）** | 已 ignore + 登记；lint allow 不等于已验证 | 安全审计 | 落地实现或作废契约 |
| Q19 历史限制 | Q19 解析/资源限制 | **候选边界（需 Owner + release notes）** | 已有修复线/对账记录 | G1 | 披露 + 复证 |

> 未列入本表的 P0 或新开 P0：按同一口径补录；**不得因“没列”而视为豁免**。

---

## 3. 晋级顺序与退出判据（提案）

| 步 | 动作 | 退出判据（可证伪） | 失败回滚点 |
|---|---|---|---|
| 1 | 统一阻断台账：把 §2 与 168 门禁、看板、STAGE.yaml 对齐 | 每条有 owner/阶段/证据位置；无重复/无漏项；分母仍 168 | 台账不一致 ⇒ 停在 ALPHA，不启动 Beta |
| 2 | 恢复 CI 执行面与审计证据链 | 8 个目标 job 真执行且有 rc；postcheck 成功；`check_audit_manifest.sh` 绿或每条例外有 Owner 裁定 | 任一 CI job 未执行 ⇒ 不用本地 run 冒充 |
| 3 | 修产品正确性阻断：IDX-2、崩溃边界、G1 wire、Q19、#5428 | 对应门禁 PASS；失败用例可复现且修复后转绿；无未裁数据正确性缺陷 | 红项未闭 ⇒ 不切 RC |
| 4 | 覆盖率 + 全量测试 + SOAK 决策 | workspace/core ≥85%（GA）或 ≥80%（RC）；全量测试 rc=0 或逐项 NOT-RUN 披露；SOAK 有效结果或 Owner 书面适用裁定 | 覆盖率不足 ⇒ 停在 RC 前 |
| 5 | Alpha→Beta→RC 逐级评审 | 每级：归档 run + hash + rc + 干净检出复现命令 + 独立 Reviewer 回执；RC tag `v4.1.0-rc1` | Reviewer 拒 ⇒ 不打 tag |
| 6 | 建 `release/v4.1.0` 并做 GA 判定 | GA required_gates 全 PASS/合法例外；merge_policy 审批齐全；Owner 批准 | 任一不满足 ⇒ 不宣布 GA、不合并 main、不打 tag |

## 4. 最小 GA 证据集（六类）

1. **阶段合法性**：ALPHA→BETA→RC 的逐级记录；RC tag；`release/v4.1.0`；GA 分支匹配。
2. **产品正确性**：IDX-2、崩溃恢复、G1 等强制测试/门禁 PASS。
3. **质量覆盖**：workspace/core ≥85%；lint/fmt/test 强制项 PASS。
4. **CI 有效性**：真实 CI run（不是本地 run）+ postcheck 证据。
5. **审计可信性**：归档 run + `evidence_hash` + rc + 干净检出复现 + 冲突处置。
6. **独立审批**：2/3 reviewers、codeowner、governance、human-architect 的有效回执。

## 5. 168h SOAK deferral 建议措辞（专家建议，待 Owner 采用）

> 现状：Owner 2026-11-13 裁决「推迟」；它不是 PASS，也未写明适用范围。
> 专家建议把延期写成「限阶段、限期限的风险接受记录」，并明确：

1. **适用范围**：168h SOAK 暂列 `DEFERRED`，**仅适用于 ALPHA→BETA** 的阶段评审材料；
   不代表验收通过，不自动适用于 RC/GA。
2. **证据状态**：SOAK 证据不足的维度保持 `INCOMPLETE`（或原始 `FAIL`）；
   延期不改变原始测试结果，不计入 PASS 数量。
3. **终止条件**：进入 RC/GA 评审前必须完成适用验收标准要求的 SOAK，
   或提交新的、明确写明适用阶段的新裁定；未满足时相应出口保持 `HOLD`。

最小可接受证据：Owner 2026-11-13 决定原文引用、适用版本与阶段、延期理由、已知风险、
责任人、补测触发条件、168h 测试协议与验收指标。

> 本文件不改 `STAGE.yaml`；正式措辞由 Owner 采用后再入 SSOT。

## 6. 本文件不做什么

- 不改变 `STAGE.yaml` 的 `current_stage`；
- 不替 Owner 给任何 P0 豁免；
- 不把“披露”写成“通过”；
- 不新增/删减门禁分母（仍 168）。
