# TF-01 启动条件核查与实施准备（BLOCKED_AUTH）

provenance: source_agent=DSH-B; source_run=TF01-PREP-20261121-R3; timestamp=2026-11-21T08:05:00+08:00; evidence_hash=见 §6; conflict_resolution=设计包冻结态哈希与属主指令给定值逐项一致；未发现第三方修改；实施被运行态授权缺失阻断，非技术阻断。

## 1. 设计固定态核验（属主指令 §一，全部实测通过）

| 项 | 期望 | 实测 | 结论 |
|---|---|---|---|
| design_branch | `codex/test-factory-design-20261121` | 存在于 gitea252 并已 fetch | ✅ |
| design_head | `cf7e9de880ac926104131952a9ba784063b9f94d` | `git cat-file -t` = commit；commit 时间 2026-11-21T07:13:00+08:00，msg「docs(v4.1.0): 冻结 Test Factory 正式设计交接」 | ✅ |
| manifest_sha256 | `4c1ee28ad8b47a8bef915bd8ca65705eadf6bf8fce52eb620ceee025a44b5c9c` | blob sha256 完全一致 | ✅ |
| SHA256SUMS | — | 在 design_head 的 tree 上 `shasum -c` **16/16 OK, rc=0**（含新增 `15_R1_ARTIFACT_DRIFT_AND_PROPAGATION.md`） | ✅ |
| PR #5923 | — | state=open, head=`cf7e9de880`, base=`develop/v4.1.0`, author=openclaw, 18 changed files（17 设计文件 + 计划文档） | ✅ |
| 基线锚点 | `e9ff21fca3` / `45da6968ae` / `b681b74ca3` | 均为本地对象；base 是 design_head 祖先 | ✅ |
| 冻结 manifest 语义 | — | `status=DESIGN_FROZEN_IMPLEMENTATION_NOT_AUTHORIZED`；`authority.implementation=false`, `worker_dispatch=false`, `ci_or_gate_change=false`, `pr_merge=false`, `stage_change=false`；`design_manifest_resign=true`；`files_digest=44c72a4b1857a6231f5c9890dbbfb73a9247d3c7f94a50eba0a377eb849da814`；`verified_red_count=null` | ✅ 设计已冻结，实施仍未授权 |

先前的"设计包未提交/未 push、files_digest=PENDING"风险已随本次冻结关闭。

## 2. DSH-A 运行态六项启动条件（写入前置门槛）

| 必需项 | 实测 | 状态 |
|---|---|---|
| TASK_ACCEPTED | 总线无此类消息；`tasks` 表 TF 检索 **0 命中** | ❌ MISSING |
| RESOURCE_GRANTED | `delegation_uses` 04:30 后 **无新增**；无 TF 相关 verdict | ❌ MISSING |
| 预算引用 | `budgets` 仅 `DSH-B-SRG-CALPHA01 granted=400000`（属 C-ALPHA-01，未对 TF-01 释放）；`09_DAG` 全部 `token_budget=OWNER_TO_AUTHORIZE` | ❌ MISSING |
| allowed_paths | 未持久化到运行态（仅存在于设计 DAG 文本） | ❌ MISSING |
| HUMAN_GATE 引用 | `OBL-TF01-AUTH` **status=BLOCKED, owner_agent=OWNER**（06:57 前后由 DSH-A 建单，等属主释放） | ❌ MISSING |
| Reviewer 安排 | `OBL-REVIEWER-DISPATCH` status=`IMPLEMENTED_NOT_VERIFIED`（owner=DSH-A），未指向 TF-01 | ❌ MISSING |

依属主指令 §一 与 `13_IMPLEMENTER_INSTRUCTIONS.md` 第 9 条：**结论=BLOCKED_AUTH**。本轮不创建实现分支、不向仓库写实现文件、不派发 Worker；仅按 §四 生成规范工件、逐 PR 补丁计划与固定 diff（未 push）。

我此前发出的 `V78-TF-R1` / `V78-TF-R2` 已由 DSH-A `ack`（06:57:30）但 `outcome=null`、未 consume。

## 3. R1 逐 PR 冻结矩阵（§四 要求，全部实测 blob sha256）

| PR | head_sha | ca01_commit_frontier.rs | ca01_backend_caps.rs | R1 状态 |
|---|---|---|---|---|
| #5918 | `d0b1157b622348aa5edb26f850f013894552f27e` | `72f7afda397e08c5…`（修订版） | 文件不存在 | CLEAN（仍受 ADR/HUMAN_GATE 阻断） |
| #5919 | `334ccf423ad839f66ea7e91c9a907fbc464c5530` | `c688a673aaefda51…`（**恒真版**） | `8d1b99397a589e5d…`（修订版，内含恒真 `c01_09c`） | **R1_DRIFT** |
| #5920 | `3949d6c1be5fe25663fd34485b4295d13532b679` | `c688a673aaefda51…` | `ca856f255bd36da8…`（**空函数版**） | **R1_DRIFT** |
| #5921 | `bfafe98d8b017840099e7aa5dae1944bf7945870` | `c688a673aaefda51…` | `ca856f255bd36da8…` | **R1_DRIFT** |
| #5922 | `ec1bb97cb3d93a3bc6d3f77af97aa0f00b9abb05` | `c688a673aaefda51…` | `8d1b99397a589e5d…`（含恒真 `c01_09c`） | **R1_DRIFT** |

case_id 集合与身份别名（`assertion_semantics` 摘要）：

- 仅 #5918 使用设计正式名 `c01_07c_paired_frontier_observation_must_exist`（静态成对观测核对，注入可翻转）。
- #5919/#5922 用旧名 `c01_07c_durable_frontier_concept_absence_documented` + `type_name::<FileStorage>()` 恒真断言 ⇒ **零判别力**。
- #5920/#5921 额外把 caps 用旧名 `c01_09a_capability_descriptor_absence_documented` / `c01_09c_wrapper_capability_honesty_observed`（**空函数体，零断言**）。
- 结论：设计 04 的正式 case 名与实际分支函数名存在**身份别名错位**（与 `OBL-NAMING-DEFECT` 同源）；15 号文档第 5 条要求的 `source_ref`→PR HEAD+blob SHA 绑定必须在 TF-01 证据中补齐。

## 4. 现存工件的三类可确证缺陷（源码实测，非推断）

1. **缺前置 → 伪装成功**：修订版 `c01_07c` 在 repo root 解析失败时 `eprintln!("SKIP: …"); return;`；三个源码 target 均不存在时 `continue` 后两个观测集皆空 ⇒ 断言 vacuous PASS。修订版 `c01_09a` 在 `engine.rs` 缺失时同样 SKIP+return。直接违反属主指令 §三.1「源码、入口、fixture 或运行条件缺失时必须输出 NOT_RUN 并使聚合结果非成功」。**这正是 §五.1 要求的「缺源码后 SKIP」负向 mutation 会抓到的缺陷。**
2. **恒真 None 冒充覆盖**：`c01_09c_wrapper_capability_attestation_must_be_observable` 在两个"修订版"里都是 `let wrapper_attestation: Option<fn() -> bool> = None; assert!(is_none())` ⇒ 恒真，即属主与设计认定的 TF-S1-003 `INEFFECTIVE_TEST`。
3. **Gap Probe 无机器可读出口**：无任何 `NOT_IMPLEMENTED` / `NOT_RUN` 标记输出，TF-03 分类器将无信号可用，只能靠退出码猜（违反 02 协议「禁止只用退出码」）。

## 5. 规范 TF-01 工件与固定 diff（已生成，未套用未 push）

规范工件（单一来源，供 4 个 R1_DRIFT PR 共用）：

- `ca01_commit_frontier.rs` 规范版 sha256=`7a425f999e6debb91b6345ba934931b4518a86736e04c59ca03281e1ba0552d4`
- `ca01_backend_caps.rs` 规范版 sha256=`06ca54b6f03b30f99f84b3c8067920b044c4ddb6f3deae564e4773279dba789e`
- 固定 diff（仓库相对路径，可 `git apply -p1`）：
  - `ca01_commit_frontier.rs.patch` sha256=`4fd1af2318a922c0e6f8cf9d30dd83c31ba4e2808cfe691b8571281d32ae79b5`（214 行，基于 #5920 head）
  - `ca01_backend_caps.rs.patch` sha256=`c8de85873146dec650168de01932ea8dfae1dda2b130b4d06d9f9974e9f80291`（110 行，基于 #5920 head）
  - 位于 `/tmp/tf01/patches/`（临时准备区；获授权后迁入 `docs/releases/v4.1.0/refactoring/evidence/test-factory/TF-01/` 并提交）

规范工件要点（严格遵循 02/04/15）：

- Gap Probe（TF-S1-001/002）：保留真实源码静态核对判据（注入即翻转，判别力已由注入实验证明）；PASS 前输出 `TFPROBE|case_id=…|kind=GAP_PROBE|result=NOT_IMPLEMENTED|contract_satisfied=false`；前置缺失改 `panic!("TFPROBE|…|execution=NOT_RUN|reason=…")` ⇒ 聚合非成功；文件头显式声明"成功只证明缺口被观察，不证明契约满足"。
- Contract Test（`c01_07a/07b/09b`）：**逐字保留**，不删除、不弱化、不 ignore（EXPECTED_RED 已登记）。
- TF-S1-003：恒真断言体作废并替换为 `execution=NOT_RUN` 出口，附 `counts_toward_coverage=false`；旧身份与旧名保留为 alias 登记在证据中（不删历史）。
- 不新增产品 API、不触碰 `src/`、`crates/*/src/`、CI、门禁、STAGE、基线。

### 覆盖退出与基线门禁的冲突（需 DSH-A/Owner 裁决，二选一）

- **方案 A**：把 `c01_09c` 从 `#[test]` 移出 ⇒ 有效覆盖真正退出，但 `tests/baseline/test_count.json` 计数下降，触发 `check_test_count_monotonic.sh` FAIL；而该脚本现无 CHECK/UPDATE 分离（TF-05 范围）⇒ 需先授权 TF-05 或显式 UPDATE_BASELINE。
- **方案 B（本轮采纳，零基线触碰）**：保留 `#[test]` 身份，出口为 `NOT_RUN` panic + `counts_toward_coverage=false` 标记，由 TF-03 分类器排除其覆盖贡献；计数不变，不动基线与门禁。
- 两方案都不删除历史源码，均符合「不得删除/ignore/弱化 RED」（`c01_09c` 本身不是 RED，而是 INEFFECTIVE_TEST）。

### §五 负向 mutation 计划（TF-01 实跑序）

M1 仅注入目标函数名 → 期望仍非 GREEN；M2 仅注入字符串/注释 → 仍非 GREEN；M3 固定 `None` 探针 → 判 `INEFFECTIVE_TEST`；M4 虚报 capability（注入 `capabilities()` 谎报 transactions=true，行为不变）→ `c01_09b` 仍 RED；M5 移走源码 target → 必须 `NOT_RUN` 且聚合非成功（**现行工件会伪装 PASS ⇒ mutation 必捕获**）；M6 空 test target 零测试 exit 0 → `NOT_RUN`。全部在隔离 worktree 执行，不触产品源码。

### 预判 BLOCKED_PRODUCT_CHANGE_REQUIRED 清单（不越界实现）

TF-S1-001 正向契约需要成对 `durable_lsn`/`commit_lsn` 观测入口；TF-S1-002 需要 `capabilities()` 描述符 + 写前 typed Unsupported；TF-S1-003 需要 wrapper attestation 出口。三者产品侧均不存在 ⇒ TF-01 出口只能是「分类正确化 + 假覆盖消除 + mutation 证据 + R1 整改」，正向契约转 `BLOCKED_PRODUCT_CHANGE_REQUIRED` 提交 HUMAN_GATE 材料，不自行实现。

## 6. 规范工件预演与 M5 mutation 对照（本轮实跑，隔离 detached worktree，未 commit/未 push）

准备区：`git worktree add --detach .worktrees/tf01-diffcheck dsh-b/c01-10-ca01-complex-dml`（**无分支名**，用后 `git worktree remove`，已清理）。
共享编译缓存：`CARGO_TARGET_DIR=.worktrees/dsh-b-c0109/target`。`git apply --check` → OK；套用后仅 2 个测试文件被改（`git status` 实测），产品源码零改动（`PRODUCT_SRC_INTACT=YES`）。

### 6.1 预演实跑（规范工件 @ #5920 head，rc=101）

| case | 实测 | 语义 |
|---|---|---|
| `c01_07a` | FAILED（left=[] right=[[1,10],[2,20]]） | Contract Test，EXPECTED_RED 保持未弱化 |
| `c01_07b` | FAILED（left=[[1,1],[2,2]] right=[]） | Contract Test，EXPECTED_RED 保持 |
| `c01_07c` | ok + 输出 `TFPROBE\|case_id=TF-S1-001\|kind=GAP_PROBE\|result=NOT_IMPLEMENTED\|contract_satisfied=false` | Gap Probe 显式非契约出口 |
| `c01_09a` | ok + 输出 `TFPROBE\|case_id=TF-S1-002…result=NOT_IMPLEMENTED` | Gap Probe 显式非契约出口 |
| `c01_09b` | FAILED（non-tx backend accepted begin_transaction, #5914） | Contract Test，EXPECTED_RED 保持 |
| `c01_09c` | FAILED + `TFPROBE\|case_id=TF-S1-003\|kind=INEFFECTIVE_TEST\|execution=NOT_RUN\|counts_toward_coverage=false` | 恒真断言已退出有效覆盖 |

两个 target 均 `0 ignored`；`ignored` 未增加。cargo `rc=101` **未被**用作 EXPECTED_RED 判据（02 协议要求）：EXPECTED_RED 由身份+签名判定，NOT_RUN 由 `TFPROBE` 标记判定。

### 6.2 M5 mutation 对照：源码 target 缺失（决定性证据）

只改测试文件内的读取路径（`crates/storage/src/engine.rs` → `…_DOES_NOT_EXIST.rs`；frontier 三 target 同改），产品源码零改动，可编译。

| 工件 | 结果 |
|---|---|
| **规范版** | `TFPROBE\|…\|execution=NOT_RUN\|reason=engine.rs not found …` ⇒ **FAILED**（`0 passed; 1 failed`）⇒ 聚合非成功 ✅ |
| **旧修订版**（caps `8d1b9939`、frontier `72f7afda`，即 #5919/#5922 现役工件） | `SKIP: engine.rs not found …` ⇒ **ok / PASS**（`1 passed; 0 failed`）❌ **伪装成功确认** |

⇒ 属主指令 §三.1 违规项与 §五.1「缺源码后 SKIP」负向 mutation 已实测成立：
verdict=**VERIFIED_DEFECT**，evidence_class=**observed**，invariant_id=**SRG-GAP-PROBE-SKIP-FALSE-PASS**，base_sha=`e9ff21fca3…`，reproduction 见 §7。
日志：`/tmp/tf01/mutation-m5.log` sha256=`46c2dbe7449d35cc01c7d4305612e65b0ec3080a86c923fc39643544d60fb2b7`；预演日志 `/tmp/tf01/mutation-run.log` sha256=`088d68caa58202f8a44f3bd4b70f4ff66175331078026457d8e9b6c358fc90f3`。

### 6.3 未运行的检查（诚实登记）

M1（仅注入函数名）、M2（仅注入字符串）、M3（固定 None 判 INEFFECTIVE_TEST 的分类器断言）、M4（虚报 capability）、M6（空 test target 零测试）：**NOT_RUN**（需在 TF-01 授权后随分类器 fixture 一起做，M3 的行为出口已就位）。
GitNexus impact / detect_changes、AOCI maintain：对正式写入的强制要求，本轮无正式写入 ⇒ **NOT_RUN**（实施时执行）。

## 7. 证据哈希

- design blob manifest `4c1ee28a…`；SHA256SUMS tree 校验 rc=0
- 旧工件：frontier 恒真 `c688a673aaefda51b5023f0f3ad3d3a6846e3cee84a784820ed875252c2f1525`；frontier 修订 `72f7afda397e08c565c5d89928dc5bd629d46adbe4a27880b6e7c958dac1a16d`；caps 空函数 `ca856f255bd36da825a1c1dd67a0c2a67c115263db7f6fecbde37e1405f234d9`；caps 修订 `8d1b99397a589e5d588dd3fcf9a1e197b82aa14ce7f140bb855d0b4d88c0d6da`
- 规范工件与 patch 哈希见 §4/§5
- 预演与 M5 日志哈希见 §6.2；本报告文件自身哈希在 bus 消息中给出（避免自引用）

## 8. 下一步（等待 DSH-A 释放）

1. DSH-A 持久化：`TASK_ACCEPTED(TF-01)` + `RESOURCE_GRANTED` + 预算引用（引用 `DSH-B-SRG-CALPHA01` 或新建）+ allowed_paths + `HUMAN_GATE` 引用 + Reviewer 安排（须非 252/8021 同族）。
2. 释放后 DSH-B：建 `.worktrees/dsh-b-tf01` + 分支 `dsh-b/tf01-s1-classification`（base=`e9ff21fca3`）→ 按 §五 顺序先跑 M1–M6 负向 mutation → 落规范工件 → 正向复跑 → 逐 PR 整改（#5919/#5920/#5921/#5922 各一 commit，仅替换两文件，保留其余增量）→ 逐 PR 复算三哈希 → GitNexus impact/detect_changes + AOCI maintain → 单 commit → 是否 push/PR 依该包运行态授权执行。
3. 完成即停：发 `REVIEW_REQUIRED` 给 DSH-A，不自签验收；TF-02 仅在四条件同时满足后启动（属主指令 §八）。

## 9. 复现命令

```bash
cd /Volumes/workspace/dev/sqlrustgo
git fetch gitea252 'refs/heads/codex/test-factory-design-20261121:refs/remotes/gitea252/codex/test-factory-design-20261121'
git cat-file blob cf7e9de880ac926104131952a9ba784063b9f94d:docs/releases/v4.1.0/refactoring/test-factory-design/14_HANDOFF_MANIFEST.json | shasum -a 256
git ls-tree --name-only cf7e9de880ac926104131952a9ba784063b9f94d:docs/releases/v4.1.0/refactoring/test-factory-design
for b in dsh-b/c01-09-backend-caps dsh-b/c01-10-ca01-complex-dml dsh-b/c01-11-ca01-bufferpool dsh-b/c01-14-ca01-visibility-oracle; do
  echo "$b $(git rev-parse $b)";
  for f in ca01_commit_frontier.rs ca01_backend_caps.rs; do
    git cat-file blob "$b:tests/integration/test_factory/$f" 2>/dev/null | shasum -a 256; done; done
cd .worktrees/tf01-diffcheck && git apply --check /tmp/tf01/patches/*.patch   # 只读校验，不 commit
```

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
