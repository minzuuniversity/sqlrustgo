# BASELINE FOLLOWUP —— Issue #1926 只读分析汇总（T-A…T-I）

> 本文件为 GitHub issue #1926 汇总评论的入仓副本（评论：https://github.com/minzuuniversity/sqlrustgo/issues/1926#issuecomment-7172057424）。入仓经用户明确授权（2026-11-25），与 #1926 §0 C1/C3 及 `AGENTS.md` §3.2/§8 #11 冲突，属于记录在案的例外；内容未改，仅把证据包路径改为仓内路径。

**执行者**：Claude Code CLI（`claude-opus-5-5`）@ `gyly@ubuntlenovoair`　**分析树**：`e976ca8515738500f03ce7988d3f55caf52c31ba`（scratch worktree，未改任何文件）　**BASE**：`fd849ae2…`
**source_run**：`baseline-followup-20261125-001`　**证据包**（已入仓）：`docs/releases/v4.1.0/evidence/baseline-followup-20261125-001/`，`evidence_hash = d9e041d162fab791bd8189848aedd1e102983829403c56df980d4156fbff4a1a`（复算规则：按文件名排序的 `path<TAB>sha256` 行逐行 LF 后取 sha256，不含 MANIFEST.yaml）
**§0 自检**：无 push、无分支、无 PR、无文件修改。唯一的写操作是本条评论。为了做 T-A 执行了 `git fetch --unshallow`（只读，允许项）。
**降级声明**：`degraded: gitnexus-missing`（无 `.gitnexus/`），全部改用 `rg`。本机无 `cargo`/`rustc`。

### 总表

| 任务 | 判定 | 产出位置 | 需属主授权？ |
|---|---|---|---|
| T-A ignore 漂移定位 | **MEASURED** | 下文 T-A；`TA_*.stdout` | 是（改 registry 或移除 `#[ignore]`） |
| T-C 文档订正补丁 | **MEASURED**（T-5 已在 `e976ca8` 中修复） | 下文 T-C | 是（落盘） |
| T-G issue 代位取证 | **MEASURED**（仓内部分）/ **ENV_UNKNOWN**（开关状态） | 下文 T-G | 否（只读查询由属主执行） |
| T-B R0 盘点 | **MEASURED** | 下文 T-B；`TB_*.stdout` | 否 |
| T-H 负向夹具复核 | **MEASURED**（静态）/ **ENV_UNKNOWN**（cargo 部分） | 下文 T-H | 是（新增夹具） |
| T-D frozen 分支 | **MEASURED** | 下文 T-D | 是（镜像写入） |
| T-E 镜像矩阵与 CI 坏因 | **MEASURED** | 下文 T-E；`TE_*.stdout` | 是（改 Actions 策略） |
| T-F 证据包与 UNKNOWN 刷新 | **MEASURED** | 下文 T-F；`MANIFEST.yaml` | 是（入仓） |
| T-I 报告事实订正 | **MEASURED** | 下文 T-I | 是（落盘） |

---

### 任务 T-A — 判定: MEASURED
- 结论:
  - 第 136 条是 `crates/storage/tests/measure_row_at_a_time_5507.rs:59`：`#[ignore = "measurement, not a gate — run with --ignored --nocapture"]`，修饰 `fn measure_row_at_a_time_commit_cost_5507`。
  - 引入它的提交是 `9abf5e5142335723744c0face28184cdfbb16ef9`（2026-11-12，omp (macmini)）：`test(storage): row-at-a-time commit cost measurement harness (#5507)`。
  - 它在 PR **#5760** 的合并提交 `66fedc290e55`（2026-11-22T18:01Z）进入 `develop/v4.1.0`。
  - 双向差集：**树多 registry 少** = 1 条（即上面这条）；**registry 多树少** = 0 条。registry 的 135 个条目与 `total_allowed=135` 自洽。
  - 二分边界（同一计数规则）：`3590e3b21`（#5760 第一父）树内 135 个 → `66fedc290`（#5760 合并）树内 136 个。
- 两个方向（只给证据，不给改法）：(a) 在 registry 里补登记这一条，`total_allowed` 135 → 136，仍低于 `total_allowed_max=150`，**不触及上限**；(b) 去掉该 `#[ignore]`，改用其他方式让测量不进普通 `cargo test`。两者都不违反 ADR-008「为了过绿而改上限」。
- 命令与退出码:
  - `python3 -I tools/ignore_diff.py <wt>`（逐字复用 `scripts/gate/common.sh:258` `count_real_ignore_attributes` 的正则与过滤规则）→ 0
  - `git log -S'#[ignore' -- <file>` → 0
  - `git log --first-parent -- <file>` → 0
- 证据: `TA_ignore_diff.stdout` sha256:`4abdd4b53da18a0a`；`TA_bisect.stdout` sha256:`83d094a5d043c7fe`；`TA_file_log.stdout` sha256:`0e882aae73f460d0`；`tool-ignore_diff.py` sha256:`680f410fcd801dab`
- 未决/受阻: 无。另见 T-H 的「执行面缺口」：fx02 证明门禁能抓到这类漂移，但 #5760 照样合入了。

### 任务 T-C — 判定: MEASURED
- 结论:
  - **T-1**：`AGENTS.md:13` 和 `README.md:14` 都写着 Cargo version 是 `3.12.0-fix-zombie`，而 `Cargo.toml:61` 实际是 `version = "4.1.0"`。补丁如下：
    ```diff
    - 2. **Version Truth**: 当前事实以 `docs/releases/v4.1.0/STAGE.yaml` + 仓库 git HEAD 为准; Cargo.toml `version = "3.12.0-fix-zombie"` 是有意保留的发布口径 (README §0 已说明)。
    + 2. **Version Truth**: 当前事实以 `docs/releases/v4.1.0/STAGE.yaml` + 仓库 git HEAD 为准; 根 `Cargo.toml` 当前为 `version = "4.1.0"`（`Cargo.toml:61`），发布口径仍以 tag / STAGE.yaml 为准。
    ```
    ```diff
    - > 注：根 `Cargo.toml` 的 `version` 仍为 `3.12.0-fix-zombie`，五个 remote 一致，属
    - > 有意保留的现状（发布口径以 tag / STAGE.yaml 为准），未随文档口径一并改动。
    + > 注：根 `Cargo.toml` 的 `version` 当前为 `4.1.0`（`Cargo.toml:61`）；发布口径仍以 tag / STAGE.yaml 为准。
    ```
    「五个 remote 一致」这句没有保留：本机只能核验 GitHub 和 gitee 两个远端，不新增事实主张。
  - **T-4**：`docs/gitea-only.md` 不存在，被 `AGENTS.md:61` 和任务文档 `:34/:71/:226` 引用。**建议改引用**，不新建文件。理由：新建文件等于新增一份治理规则，按 DOC_CHECK §2.1 最小修改原则，规则本体已经在 AGENTS §3.2/§8 #11 里。补丁：
    ```diff
    - **严禁** `gh` CLI 或 GitHub。详见 `docs/gitea-only.md` + 项目 CLAUDE.md。
    + **严禁** `gh` CLI 或 GitHub。详见本文件 §8 #11（`docs/gitea-only.md` 与根 `CLAUDE.md` 在本仓库中不存在）。
    ```
    `docs/plans/2026-11-26-…-task.md` 的三处引用是历史任务书，建议不改，只在 PR 里注明。如果属主更想新建文件，骨架是：标题、适用范围、远端表（照抄 `scripts/sync/README.md:8-9`）、禁止项（照抄 AGENTS §8 #11）、例外的审批路径（本轮 #1925/#1926 的先例）。
  - **T-5**：已在 `e976ca8` 中修复。`V410_REMOTE_BASELINE_ALIGNMENT_TASK.md:122/:315/:338` 已改为「没有 `--check`，用 `--out` 指到仓外再比对 `summary`」。只剩 `:359` 是对远端实测的引述，属于正确的历史陈述，**不需要补丁**。
  - **新发现的口径不一致（只报告，不出补丁）**：`AGENTS.md` §8 #12 和 §9 说文档改动走「5 步流程」，同文件 §10 和 `DOC_CHECK_CORRECTION_RULES.md:57` 是「7 步标准流程」。
- DOC_CHECK 检查单（`DOC_CHECK_CORRECTION_RULES.md` §四 4.1–4.5）：修改正确性（每处补丁都对应 `Cargo.toml:61` 或文件缺失的实测）；无过度修改（只改句子，不改章节结构）；链接有效性（删掉了指向不存在文件的链接）；git 状态（单个 commit）；可撤销性（`git revert` 即可）。完整流程是 7 步，步骤 6 实跑 `check_docs_links.sh`，需要落盘时执行。
- 命令与退出码: `rg -n '3.12.0-fix-zombie' AGENTS.md README.md` → 0；`rg -n '^version' Cargo.toml` → 0；`rg -n 'gitea-only.md'` → 0；`rg -n -- 'gen_verdict.py --check'` → 0
- 证据: 行号以 `e976ca8` 为准，可用同一组 `rg` 复算
- 未决/受阻: 落盘需属主授权（O-C）

### 任务 T-G — 判定: MEASURED（仓内可证部分）/ ENV_UNKNOWN（Gitea open/closed 状态）
- 结论:

| issue | 仓内可证 | 必须 252 确认 |
|---|---|---|
| #4914 | B1.1/B1.2 已合入：PR #4927（`d0c2403ef`，2026-10-01）；后续 #5010（`7168c9a7b`）、#5037（`eddd225ac`）。**B1.3 搁置**：`docs/releases/v4.1.0/PERFORMANCE_OPTIMIZATION_PLAN.md:555`「B1.3 判定为独立设计 PR」、`:602`「⏸ 暂不实施」，没有任何提交实现 B1.3 ⇒ REF-01 ⇐ B1.3 这条硬边（`STAGE.yaml:911`）**仍然阻塞** | issue 的 open/closed 状态 |
| #4947 | 已合入 PR #4962（`7c3dbe10d`，2026-10-04），随后**被回退**：`f4562023e`「revert trigger/stored_proc to scan() — scan_with_filter(&\|_\| true) is 4% slower (bench-backed)」；代码注释 `crates/executor/src/trigger.rs:892`、`stored_proc.rs:1752`。`STAGE.yaml:845` 记录的「当前编译不过（Self: Sized）」已过时 ⇒ REF-03 ⇐ #4947 这条硬边的前提**需要属主重新确认** | 是否已经关闭 |
| #4708 | `ISSUES_PLAN.md:109`：Gitea 曾在 2026-09-03 关闭（PR #4746），复验「❌ 未修复，块注释 parser 完全不支持」，状态 **RE-OPENED**；`STAGE.yaml:115` 为 `DONE_REOPENED_2026-09-30` | Gitea 上是否真的被重新打开 |
| #4696 / #4710 / #4720 | `ISSUES_PLAN.md:110-112`、`STAGE.yaml:123-125`：Gitea 于 2026-09-03 关闭；复验判 **PARTIAL**（parser 回归通过，E2E 未验证） | 同上 |

  - **仓内互相矛盾**：`LEGACY_ISSUES.md:16` 仍写 WP-A「DONE (inherited DONE)」，与 `STAGE.yaml:105-115` 的 `DONE_REOPENED` 相反（这是旧矛盾，STAGE 已登记）。`STAGE.yaml` 的 `progress.wave_1..4: "⬜ 未开始"` 与上面 #4914、#4947 的合入事实相反 ⇒ 这一段已经过时。
- 只读查询模板（由属主执行）：
  ```bash
  for n in 4914 4947 4708 4696 4710 4720; do
    curl -s -K <0600 配置文件，内含 Authorization 头> \
      "http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/issues/$n" \
      | python3 -c 'import json,sys;d=json.load(sys.stdin);print(d["number"],d["state"],d.get("closed_at"),[l["name"] for l in d.get("labels",[])])'
  done
  ```
- 命令与退出码: `rg -n '#<n>\b' STAGE.yaml KANBAN_*.jsonl ISSUES_PLAN.md takeover/ LEGACY_ISSUES.md` → 0；`git log -E --grep='#<n>\b'` → 0
- 证据: 行号以 `e976ca8` 为准，可用同一组命令复算
- 未决/受阻: open/closed 状态需要 252 的 API。拿到 `read:repository` 的只读 PAT 后可以一次清零。

### 任务 T-B — 判定: MEASURED
- 结论（行数与上轮 E-10 一致：5269 / 4277 / 2627）：

| 文件 | 总行数 | **产品代码**（第一个 `#[cfg(test)] mod tests` 之前） | 内联测试 | `pub` 面 | 外部调用方（`crates/executor/src` 之外的文件数） |
|---|---|---|---|---|---|
| `expr/mod.rs` | 5269 | **1–3729**（3729） | 3730–5269（1540，118 个 `#[test]`） | 27 fn + `UdfDefinition` + `UnifiedExpr` | 7（`src/engine_select.rs`、`engine_utils.rs`、`expr_utils.rs` + 4 个测试） |
| `stored_proc.rs` | 4277 | **1–2668**（2668） | 2669–4277（1609，119 个 `#[test]`） | 43 fn + 4 struct | 18（含 `src/execution_engine.rs`、`engine_ddl.rs`、`crates/catalog/src/lib.rs`） |
| `trigger.rs` | 2627 | **1–1346**（1346） | 1347–2627（1281，74 个 `#[test]`） | 21 fn + 1 struct + 4 enum + 2 trait | 7（`src/engine_helpers.rs`、`execution_engine.rs`、`engine_dml.rs`、`crates/types/src/error.rs` 等） |

  - **关键观察**：C-ARCH-05-NOTE 用 `wc -l` 量整个文件（`check_arch_invariants.sh:151`）。三个文件里内联测试占 29–38%。**`trigger.rs` 的产品代码只有 1346 行，已低于 1600**：只要把 `mod tests` 移到 `trigger/tests.rs`（零语义变更），REF-03 就能达标。所以 REF-03 和 #4947 的硬边可能**不必**串行，需属主复核。REF-01 和 REF-02 移出测试后仍然超限（3729、2668），需要真正拆分。
  - **与 #4914 B1.3 的重叠**：B1.3 的目标是 `find_column_index`。它在 `expr/mod.rs:642` 有定义，文件内引用行是 639、642、646、704 和 4920–4990、5221、5252（大部分在测试区）。全仓有 **7 个同名或近名定义**（`src/engine_utils.rs:1027`、`src/expr_utils.rs:1471`、`crates/storage/src/engine.rs:891/919`、`crates/sql-corpus/src/lib.rs:795`、`crates/executor/src/merge.rs:454`、`expr/mod.rs:642`），在 `crates/` 和 `src/` 中共出现 133 次。计划文档写的「40 处调用点」已经过时。
  - **与 #4947 的重叠**：`trigger.rs` 的 `scan` 调用在 894、895、901、1000 行；`stored_proc.rs` 在 837、961、1033、1096、1251、1254、1752、1757、1803、1934、2007、2092 行。都在产品代码区。
- 命令与退出码: `wc -l`；`rg -c '^\s*pub (async )?fn '`；`rg -n '#\[cfg\(test\)\]'`；`rg -l -e '<k>' crates src tests --glob '!crates/executor/src/**'` → 全部 0
- 证据: `TB_r0.stdout` sha256:`a229479f8ee7047f`；`TB_prod_vs_test.stdout` sha256:`da95f958c753bd3d`；`TB_overlap.stdout` sha256:`9e2a69f02a1190ed`
- 未决/受阻: 测试热点的「覆盖率」部分需要 cargo-llvm-cov，标 ENV_UNKNOWN。

### 任务 T-H — 判定: MEASURED（静态）/ ENV_UNKNOWN（需要 cargo 的夹具实跑）
- 结论:
  1. **判据 #4/#6 没有任何负向夹具**：`NEG_FIXTURES_v4.1.0.jsonl` 共 57 行、覆盖 57 道门禁，`audit/v4.1.0/neg/` 有 22 个 fx。两者对 `gen_verdict.py`、`verify_matrix.py`、`verify_handoff.py` 的引用数都是 **0**。判据 #4 和 #5 的机制有 `verify_handoff.py:519-534` 的变异臂（cycle / missing / unapproved_dispatch / unapproved_adr / human_gate_removed / cloud_policy…），但 **`approved_adr_refs` 为空的情况没有变异臂**：`verify_handoff.py:383-384` 在 `refs` 为空时直接 `return`，R1 由 `gen_verdict.py` 的计数间接兜底（`:480` 的 `tasks_with_empty_approved_adr_refs`），而这个兜底本身没有夹具。
  2. **判据 #6 的签署人身份没有绑定**（`gen_verdict.py:528-610`）：
     - 合格签署只看三件事：`assertion_reviewed_by` 非空、它不等于 `assertion_review_performed_by`（字符串比较）、它出现在 `V410-OWNER-RULING.md` 里 Markdown 标记块解析出的名单中（`owner_approved_reviewers`，`:255-280`）。名单就是仓内的一张 Markdown 表，**没有绑定属主身份**（不校验 commit 作者或签名）。`CODEOWNERS` 和 `.github/CODEOWNERS` 也**没有覆盖** `docs/refactor/**`（`rg 'docs/refactor|OWNER' CODEOWNERS` 没有命中）。
     - **可构造的绕过路径**：`judged_pass` 排除了 `ASSERTION_UNREVIEWABLE`。如果把 43 条标成 UNREVIEWABLE，剩下 1 条由名单内的任意字符串签署，6 个 `cond6` 就全部为真。这里缺两个夹具：「签署人名单被非属主改动」和「UNREVIEWABLE 比例异常」。
  3. **执行面缺口（T-A 的成因）**：fx02 证明 `check_anti_ignore_gate.sh` 能抓到未登记的 `#[ignore]`，但 #5760 仍然合入了。原因：`.gitea/workflows/` 中带 `pull_request` 触发、覆盖 develop/v4.1.0 的只有 `ci.yml` 和 `gate.yml`，`ci-alpha-stage.yml` 只能手动触发（`workflow_dispatch`）。252 上 `develop/v4.1.0` 要求哪些状态检查，本机查不到（**ENV_UNKNOWN**）。结论：**夹具证明了「门禁能红」，没证明「红了就挡得住合并」**。
- 命令与退出码: `ls audit/v4.1.0/neg/`；`rg -c -i 'gen_verdict|verify_matrix|verify_handoff' NEG_FIXTURES_v4.1.0.jsonl audit/v4.1.0/neg/` → 0 次命中；`sed -n 520,610p gen_verdict.py`；触发器解析脚本 → 0
- 证据: `TH_gitea_triggers.stdout` sha256:`f1423b715cf684ce`；行号以 `e976ca8` 为准
- 未决/受阻: 22 个 fx 的实跑需要 cargo（ENV_UNKNOWN）；252 的分支保护 status checks 需要 252 API

### 任务 T-D — 判定: MEASURED
- 结论:
  - `base/v4.1.0-frozen` = `936610b6d08a`，**是 `develop/v4.1.0`（`e976ca8`）的祖先**：base 独有提交 0 个，develop 独有 28 个。BASE `fd849ae` 是它的祖先。它的内容已经通过 #5928–#5932 合入 develop。
  - **镜像缺这个分支是同步工具范围造成的遗漏，没有声明说是有意不同步**：`scripts/sync/5remotes_sync.sh:85` `ALL_BRANCHES=("develop/v4.1.0" "main" "release/v4.0.0")` 不包含它。`V410_BASELINE_FROZEN_DECLARATION.md:32` 只写「已推送远端」，没有指明哪些远端，也没有排除 GitHub 的说明。
- 同步方案（只出方案）：在镜像上**新建**这个分支，指向 `936610b`。它是 develop 的祖先，所以是纯新增，没有非快进问题。不改 `ALL_BRANCHES`，一次性手动推送即可；要长期同步，就要改脚本（需授权）。风险：低，不影响任何已有 ref。回滚：删除这个分支。**需要属主授权**（镜像写入）。
- 命令与退出码: `git rev-list --count develop..936610b` → `0`；反向 → `28`；`git merge-base --is-ancestor 936610b develop` → 0；`--is-ancestor fd849ae 936610b` → 0
- 证据: 可用同一组命令复算
- 未决/受阻: 无

### 任务 T-E — 判定: MEASURED
- 结论（只读 API，2026-11-25T19:1xZ）：

| 仓库 | 默认分支 | `develop/v4.1.0` 保护 | `main` 保护 | Actions | 近期运行 |
|---|---|---|---|---|---|
| `minzuuniversity/sqlrustgo` | `main` | 无（404） | 无（404） | `enabled: true`，**`allowed_actions: local_only`** | 共 9493 次；最近 8 次全部 `startup_failure`（包括 `e976ca85`、`5f9048a6`） |
| `yinglichina8848/sqlrustgo-public` | `develop/v4.0.0` | 无（404） | 无（404） | `allowed_actions: all` | 能真正运行（`in_progress`，也有 `failure`） |

  - **根因链**：`local_only` 只允许本仓库内定义的 action。`.github/workflows/` 里**没有任何本地 action 引用**（`uses: ./` 计数为 0），引用的全是第三方 action：`actions/checkout@v4` ×45、`dtolnay/rust-toolchain@stable` ×25、`actions/upload-artifact@v4` ×14、`Swatinem/rust-cache@v2` ×7、`actions/cache@v4` ×6、`actions/download-artifact@v4` ×1、`benchmark-action/github-action-benchmark@v1` ×1。工作流在解析阶段就被策略拒绝，所以是 `startup_failure` 且 0 个 job。
  - **不依赖运行的验证方法**：`GET /repos/<r>/actions/permissions` 返回 `local_only`；`GET …/selected-actions` 返回 409「Only local actions and workflows are allowed」；再加上上面的 `uses:` 静态清单，三者即可构成证明。
  - 最小修复建议（只建议）：(a) 改为 `selected`，白名单只放上面 7 个 action，并要求 SHA pinning；或 (b) 如果镜像本来就不该跑 CI，关闭 Actions（`enabled: false`），停止产生大量 startup_failure 记录。**需属主授权**。
  - 附带风险：本机用的 PAT 对这两个仓库都有 **admin** 权限（`permissions.admin: true`），超出本任务只读和评论的需要。建议换成最小权限的细粒度 token（Issues: write + Contents/Metadata: read）。
- 命令与退出码: `GET repos/<r>`、`/actions/permissions`、`/actions/permissions/selected-actions`（→ 409）、`/branches/<b>/protection`（→ 404）、`/actions/runs` → 均为 HTTP 2xx/4xx，符合预期；`rg 'uses:' .github/workflows` → 0
- 证据: `TE_matrix.stdout` sha256:`2ce601f0f69fcdfe`；`TE_uses.stdout` sha256:`f57de3f100f3e1a4`
- 未决/受阻: gitcode、gitea250、252 这三个远端的保护和 CI 状态 **ENV_UNKNOWN**（252、250 网络不可达；gitcode 未查）

### 任务 T-F — 判定: MEASURED
- 结论: 证据包草案已在执行主机本地建好（**未入仓**）。目录如下：
  ```
  bundle/baseline-followup-20261125-001/
    MANIFEST.yaml  env.yaml
    r1-{arch_invariants,gate_registry,honesty,check_stage,anti_ignore}.stdout  r1-gen_verdict_check.stderr   # 上轮证据，换成了允许的扩展名
    TA_{ignore_diff,bisect,file_log}.stdout  TB_{r0,prod_vs_test,overlap}.stdout
    TE_{matrix,uses}.stdout  TH_gitea_triggers.stdout  TI_pr1925.stdout  tool-ignore_diff.py
  FILELIST.txt   # （入仓时移入证据目录并由 .tsv 改名，内容不变）排序后的 path<TAB>sha256，是 evidence_hash 的输入
  ```
  包内没有 `*.json`、`*.log`、`*.tbl`（已实测）。上轮的 `.log` 已改名为 `.stdout`，sha256 不变（例如 `r1-anti_ignore.stdout` 仍是 `6710a073a3d33482`，与上轮报告 E-9 一致）。
- **UNKNOWN 清单刷新**：

| ID | 上轮 | 本轮 |
|---|---|---|
| U-1 252 tip | UNKNOWN | **已消除**（属主确认：252 = 镜像 = `e976ca8`，§1） |
| U-2 issue 状态 | UNKNOWN | **部分消除**：仓内证据见 T-G；open/closed 状态仍需 252 |
| U-3 cargo 门禁 | UNKNOWN | **仍是 UNKNOWN**（本机无工具链，镜像 CI 起不来，见 T-E） |
| U-4 REF 作者与频率 | UNKNOWN | **可消除**（已 unshallow）；本轮按优先级没做逐行统计 |
| U-5 `sqlrustgo-public` tip | UNKNOWN | **已消除**：最近的运行在 `e976ca85`、`ac3fa16a` 上，镜像在同步 |
| U-6 ignore 漂移引入点 | UNKNOWN | **已消除**（T-A：`9abf5e5` 经 #5760） |
- 命令与退出码: `sha256sum` 逐个文件 → 0；`ls | grep -E '\.(json|log|tbl)$'` → 无输出
- 证据: `MANIFEST.yaml`、`FILELIST.txt`；`evidence_hash` 见本评论开头
- 未决/受阻: 入仓需属主授权

### 任务 T-I — 判定: MEASURED
- 结论（实测）：GitHub PR #1925 的状态是 `merged: true`，`merge_commit_sha = e976ca8515738500f03ce7988d3f55caf52c31ba`，`merged_at 2026-11-25T18:59:05Z`。`e976ca8` 的两个父提交是 `9c031dc8`（develop）和 `5f9048a6`（报告提交）。也就是说，#1925 是在 252 的合并结果同步到镜像时被 GitHub **自动识别为已合并**的，并没有在 GitHub 上单独合并。
- 可直接落盘的插入文本（放在 `BASELINE_ALIGNMENT_2026-11-25.md` 的「性质」行之后，替换原来「未使用 `gh`」那一句）：
  > **事实说明（2026-11-25 补记）**：本报告的**分析主体是只读**（只运行了只读的 git 命令、静态脚本和门禁检查，没有修改产品代码、门禁脚本或 STAGE.yaml）。报告**落盘这一步**是写操作，路径如下：执行者按用户指示，经 SSH 把分支 `docs/v410-baseline-alignment-report-20261125`（提交 `5f9048a6`）推送到 GitHub 镜像，并用执行主机上的 GitHub PAT 通过 REST API 建了 PR #1925（没有使用 `gh` CLI，但这同样是对 GitHub 的写操作）。随后属主在 Gitea 252 经 PR #5937 合入该提交，合并提交为 `e976ca8515738500f03ce7988d3f55caf52c31ba`；镜像同步后，GitHub 把 #1925 标为已合并，`merge_commit_sha` 也是 `e976ca85…`。因此 252 与镜像之间**零漂移**。上述推送和建 PR 违反了 `AGENTS.md` §3.2/§8 #11，属于经用户明确指示的例外，记录在此。
- 命令与退出码: `GET repos/minzuuniversity/sqlrustgo/pulls/1925` → 200；`git log -1 --format=%P e976ca8` → 0；`git merge-base --is-ancestor 5f9048a e976ca8` → 0
- 证据: `TI_pr1925.stdout` sha256:`2b104bcd7aa3d2d2`
- 未决/受阻: 落盘需属主授权

---
**未做**：无（9 项都已产出）。**本轮最重要的三件事**：① ignore 漂移来自 `9abf5e5` / #5760，修复是补 1 条登记或移除 1 个 `#[ignore]`；② `trigger.rs` 的产品代码只有 1346 行，REF-03 也许只需移出测试模块就能达标；③ 判据 #6 的签署人名单没有身份绑定，也没有负向夹具，是本轮发现的最大治理缺口。
