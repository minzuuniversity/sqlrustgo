# SQLRustGo v4.1.0 —— 基线对齐报告 / 重构路线图 v0.1 / Harness 治理启动包 v0.1

> **性质**：仅分析与方案。未改产品代码、门禁脚本、STAGE.yaml、分支保护；未使用 `gh`。本文件经用户明确指示以 PR 形式推送到 GitHub `minzuuniversity/sqlrustgo`（与 AGENTS.md §3.2/§8 #11「只走 Gitea」冲突，见 PR 说明）。
> **证据文件位置**：《证据索引》中的 `scratchpad/ev/*` 是执行主机上的会话临时目录，**未入仓**；sha256 前缀用于在该主机上复核。
> **任务依据**：`docs/plans/2026-11-26-claude-code-baseline-alignment-refactor-harness-governance-task.md`（sha256 `d700103b…cc9b`）
> 姊妹任务文档：`docs/releases/v4.1.0/V410_REMOTE_BASELINE_ALIGNMENT_TASK.md`（同日、更细，含 12 条准入判据与 D1–D11 复核表）

## 证据头（AFP 五字段）

| 字段 | 值 |
|---|---|
| `source_agent` | Claude Code CLI（`claude-opus-5-5`），主机 `gyly@192.168.60.14`（**不在** 192.168.0.0/24 内网） |
| `source_run` | 本会话 `2604c299-addc-4890-aa9f-840768ec473f`；证据目录 `scratchpad/ev/` |
| `timestamp` | `2026-11-25T18:23:48Z`（Step 0）～ `2026-11-25T18:25Z`（门禁运行） |
| `evidence_hash` | 见文末《证据索引》逐项 sha256 前缀 |
| `conflict_resolution` | 文档口径与实测冲突时，一律以实测为准并在「张力点」逐条列出；不改写历史记录 |

---

## Step 0 证据快照

| 项 | 实测值 | 命令 |
|---|---|---|
| 工作目录 | `/home/gyly/ai-review/sqlrustgo-v4.1.0`（**原为 gitee 浅克隆**，非 GitHub 检出） | `pwd` |
| remotes | `origin` = `https://gitee.com/yinglichina/sqlrustgo.git`；本会话**新增** `github` = `https://github.com/minzuuniversity/sqlrustgo.git`（只 fetch） | `git remote -v` |
| HEAD | `3564a8b3ecf30ccffed0f6e39fa3c138a4fcab97`（PR #5933 merge，2026-11-25T18:13:28Z，openclaw） | `git rev-parse HEAD` / `git log -1 --pretty=fuller` |
| 分支 | `develop/v4.1.0`，由 `8cbde5b` **fast-forward** 到 `github/develop/v4.1.0`（+119 commits） | `git merge --ff-only github/develop/v4.1.0` |
| 工作区 | 干净（门禁运行产生的日志已移出仓库到 scratchpad） | `git status --short` → 空 |
| 浅克隆 | `true` —— 历史只到 `8cbde5b`，git blame/作者统计不可用 | `git rev-parse --is-shallow-repository` |

**STAGE.yaml 关键行摘要**（sha256 `5b150ca5…75ed`，1189 行）：

| 行 | 内容 |
|---|---|
| L201 | `current_stage: "ALPHA"` |
| L61–84 | `inherited_from_v4000_revocation`：v4.0.0 GA **REVOKED 2026-09-30**；COV-01/02 DONE（86.20% / 87.89%）；REF-01/02/03 无 `status` 字段（=未开工） |
| L84 | `gate_policy`：C-ARCH-05 只测 `src/execution_engine.rs`；REF-01..03 由 C-ARCH-05-NOTE 报 WARN、不阻断；**扩展 C-ARCH-05 必须在 REF-01..03 落地之后** |
| L105–115 | `false_done_wp_a`：#4708/#4696/#4710/#4720，`status: DONE_REOPENED_2026-09-30` |
| L519–527 | `ga_carrier_branch: release/v4.1.0`，`ga_declared: false` |
| L868/883/911 | 硬串行边：REF-01 ⇐ #4914 B1.3；REF-03 ⇐ #4947 |
| L1131/1156 | 2026-11-18 属主签批后 `current_stage` 仍为 ALPHA；GA 发布批准待 168h SOAK 独占轮 |

---

# 一、《基线对齐报告》

## 1.1 对齐目标声明 —— 采用 **策略 A + C**

- **A（Gitea 主线基线）**：权威事实源是 `http://192.168.0.252:3000/openclaw/sqlrustgo.git`（`AGENTS.md` §3.2、`V410_REMOTE_BASELINE_ALIGNMENT_TASK.md` 头部均明示）。GitHub 仅为只读镜像。
- **C（双基线差异报告）**：本机**无法直连 252**（实测 `Failed to connect to 192.168.0.252 port 3000`），所以 A 的那一侧只能标 `UNKNOWN`。本报告只输出可核验的镜像间差异，收敛方向交属主决定。
- **不采用 B**：理由是仓库规则（AGENTS §8 #11）禁止推 GitHub。把 GitHub 当基线只会制造反向债务。
- **补充一个「内容基线」**：仓库已经声明 **BASE = `fd849ae20541901d5e59f45cd3ba2c51093f5d5e`**（`BASELINE_FREEZE.json`，经 `V410_BASELINE_FROZEN_DECLARATION.md` 转述）。在本报告里，「基线」分两层：**ref 基线**（各远端 tip）和**内容基线**（BASE）。后者不随 tip 移动。

## 1.2 Remote/Ref 对比表（2026-11-25T18:23Z，`git ls-remote`）

| ref | Gitea 252（权威） | GitHub `minzuuniversity` | gitee `yinglichina` | 本地 HEAD |
|---|---|---|---|---|
| `develop/v4.1.0` | **UNKNOWN**（网络不可达） | `3564a8b3ecf3` | `a604ed9ab339`（PR #5932） | `3564a8b3ecf3` |
| `main` | **UNKNOWN** | `feefb6330c34` | `feefb6330c34` | — |
| `base/v4.1.0-frozen` | **UNKNOWN**（声明称「已推远端」） | **不存在** | `936610b6d08a` | — |
| BASE `fd849ae2…` 是否为 HEAD 祖先 | — | — | — | **是**（对象存在；`git diff BASE HEAD -- crates/ src/` 为空） |

其他镜像：`yinglichina8848/sqlrustgo-public` 在 2026-11-25 首次查询时 `develop/v4.1.0` = `8cbde5b`，说明它落后于 `minzuuniversity`（本轮未复查）。

## 1.3 差异分类表

| # | 对象 | 分类 | 证据 |
|---|---|---|---|
| D-01 | gitee `develop/v4.1.0` 落后 GitHub 5 个合并（a604ed9 → 3564a8b：#5934、#5933 及其子提交） | **仅文档差异**＋审计归档（`audit/`、`docs/`、`refactoring/tools/*.py`） | `git log a604ed9..3564a8b --stat` |
| D-02 | `base/v4.1.0-frozen` 在 GitHub 缺失，在 gitee 存在 | **镜像覆盖差异** | ls-remote 两侧对比 |
| D-03 | 产品代码 `crates/` `src/`：BASE 与 HEAD 之间 | **一致**（diff 为空） | `git diff --stat fd849ae… HEAD -- crates/ src/` |
| D-04 | CI：`.gitea/workflows/` 9 个，`.github/workflows/` 13 个，两套文件名不重合 | **门禁/CI 差异**（两套 CI，不是镜像漂移） | `ls` 两目录 |
| D-05 | Gitea 252 上 tip 是否等于 `3564a8b` | **UNKNOWN（需人工）** | 网络阻塞 |
| D-06 | Gitea issue 状态（#4914、#4947、#4708 等） | **UNKNOWN** | 无 252 API 访问 |

## 1.4 口径张力点（实测 vs 文档）

| # | 文档口径 | 实测 | 判断 |
|---|---|---|---|
| T-1 | `AGENTS.md:13`、`README.md:14`：根 `Cargo.toml` 的 version 为 `3.12.0-fix-zombie`，「有意保留」 | `Cargo.toml` → `version = "4.1.0"` | **文档陈旧**。两处陈述与事实不符，需按 DOC_CHECK_CORRECTION_RULES 订正（未改） |
| T-2 | `STAGE.yaml:904`：「三文件行数 2026-10-04 复核未变」（5104 / 4248 / 2549） | `expr/mod.rs` 5269、`stored_proc.rs` 4277、`trigger.rs` 2627 | **三个文件都在增长**（+165 / +29 / +78）。REF 范围在扩大 |
| T-3 | `AGENTS.md:73`：GitNexus 索引存在 `.gitnexus/` | 本检出**没有** `.gitnexus/` | AGENTS §4 的 MUST 条目在此环境无法执行 ⇒ 降级到 rg + 源码 |
| T-4 | 任务文档 §10 列了 `docs/gitea-only.md`、根 `CLAUDE.md` | 两者**都不存在**；只有 `.claude/CLAUDE.md`，其中没有 Gitea-only 条款 | 「禁 GitHub」的权威出处只剩 AGENTS §3.2/§8 |
| T-5 | 姊妹任务 D11：`gen_verdict.py --check` | 退出码 2：`unrecognized arguments: --check` | **任务文档里给的复核命令本身不可执行** |
| T-6 | `AGENTS.md:197`：工作目录 `/Volumes/workspace/dev/wt-v410-base` | 那是 macOS 路径，本机不存在 | 只是环境说明，不影响事实 |

## 1.5 本机门禁实测（只读，不改语义）

| 命令 | 退出码 | 摘要 | 归类 |
|---|---|---|---|
| `bash scripts/gate/check_arch_invariants.sh` | 0 | `PASSED: 5 FAILED: 0`；`execution_engine.rs` 1268 行 < 1600；NOTE 段对三个 REF 文件报 WARN | 实测 PASS |
| `python3 scripts/gate/gate_registry.py check` | 0 | `check_n=168 catalog_rows=168 fails=0`；dead=61 | 实测 PASS |
| `python3 audit/v4.1.0/honesty.py check` | 0 | `gates=168 fixtures=57 fails=0` | 实测 PASS |
| `bash scripts/gate/check_stage.sh` | 1 | `Stage ALPHA (v4.1.0): 9 PASS, 8 FAIL` | 见下 |

`check_stage.sh` 的 8 个 FAIL 拆开看：
- **环境原因（7 个）**：本机**没有 `cargo`**（日志 `line 36: cargo: command not found`）。受影响的是 build、fmt、test --lib、anti-fab（6 个 ERROR 都源于 cargo 缺失）、coverage、corpus、tpch。这 7 个**不构成代码判定**，在本机状态为 UNKNOWN。
- **真实树级 FAIL（1 个）**：`check_anti_ignore_gate.sh` 报告「tree has **136** real `#[ignore]` attributes but registry registers **135**」。这是登记漂移，和 cargo 无关，可复现。需要定位是哪个提交引入的。浅克隆限制了二分，建议在 252 侧执行。

> 结论：本报告**不宣告**「对齐成功」。可核验的结论只有三条：镜像间 ref 差异（1.2）、产品代码对 BASE 零漂移（D-03）、ALPHA 门禁里有 1 道是真实的红（ignore 漂移）。

## 1.6 风险清单（含不可逆操作警告）

| # | 风险 | 不可逆？ |
|---|---|---|
| R-1 | 拿 GitHub tip 当权威：GitHub 缺 `base/v4.1.0-frozen`，252 侧可能还有未同步提交 | 否 |
| R-2 | 往任何镜像 push 或 force-push（AGENTS §8 #11，且镜像有 force-push 保护） | **是** |
| R-3 | T-1 文档陈旧会误导新 Agent 判断版本口径 | 否 |
| R-4 | ignore 漂移 136/135 让 ALPHA 清单在任何有 cargo 的环境里也是红的 | 否 |

## 1.7 UNKNOWN 清单

U-1 252 上各 ref 的 tip；U-2 252 上的 issue/PR 状态（#4914、#4947、#4708…）；U-3 cargo 相关的 7 道 ALPHA 门禁（本机无工具链）；U-4 REF 三文件的近期作者与变更频率（浅克隆）；U-5 `sqlrustgo-public` 镜像当前 tip；U-6 ignore 漂移是哪个提交引入的。

---

# 二、《重构路线图 v0.1》

## 2.0 准入现状（决定能做什么）

- `V410-READINESS-VERDICT.json`：12 条判据的分布为 PASS 9 / FAIL 2（#4 映射层、#6 签署层）/ WAIVED 1。数据**转引**自 `V410_BASELINE_FROZEN_DECLARATION.md`（source_agent `dsh-b`，run `20261126-criterion8-negfix-001`）。本机复算命令 T-5 不可用，所以置信度降一级。
- `V410-OWNER-RULING.md:387-398` §4 属主裁决位：**全空**（实测）。
- 因此 **REF-01..03 的产品代码拆分目前无权开工**。可以开工的只有不动产品代码的 R0/R1。

## 2.1 依赖与顺序

```
#4914 B1.3 ──► REF-01 (expr/mod.rs 5269)
#4947      ──► REF-03 (trigger.rs 2627)
REF-02 (stored_proc.rs 4277)  ── 与 #4947 同属读路径（STAGE L879），建议排在 #4947 之后
属主 O1(判据4) + Reviewer 副签(判据6) + O5 放行批次 ──► R2 任何切片
REF-01..03 全部落地 ──► R3：扩展 C-ARCH-05 覆盖（STAGE L84）
```

#4914 和 #4947 的状态在本机为 **UNKNOWN**（U-2）。

## 2.2 Phase 划分

| Phase | 内容 | 是否需要属主裁决 | 改不改产品代码 |
|---|---|---|---|
| **R0 只读盘点** | 三个文件的符号清单、`pub` 面、crate 内外调用方（rg）、测试覆盖热点、与 #4914/#4947 的重叠行区间；行数基线写进证据包 | 否 | 否 |
| **R1 契约先行** | 每个文件一份「拆分设计笔记」：子模块划分、公开 API 不变清单、`pub(crate)` 迁移、类型归属、迁移顺序、可证伪验收判据；预备特征测试（锁行为，不锁实现） | 否（但笔记进 `docs/` 要走 DOC_CHECK 5 步） | 否 |
| **R2 机械拆分** | 每片只搬代码不改语义：`mod.rs` → `expr/{arith,cmp,func,cast,…}.rs`；每片单独提交、单独可回滚 | **是**（O1/O5 + 判据 6） | 是 |
| **R3 门禁对齐** | 三个文件都 < 1600 后，提案把 C-ARCH-05 从单文件扩到「`crates/executor/src/**` 每个文件 < 1600」 | **是**（改门禁语义需逐项授权） | 门禁 |

## 2.3 任务验收卡片

### T-1 R0 盘点（REF-01..03 共用）
- **目标**：产出可复算的现状测绘。**非目标**：不提出拆分方案，不改任何文件。
- **影响面**：无（只读）。**风险**：Low。**回滚**：无需。
- **验证**：`wc -l` 三个文件；`rg -n '^\s*pub (fn|struct|enum|trait)'`；`rg -l 'expr::' crates src`；`bash scripts/gate/check_arch_invariants.sh` 的 NOTE 段。
- **证据**：`docs/releases/v4.1.0/evidence/refactor-r0-<date>/`（`.stdout`/`.yaml`，遵守「`*.json`/`*.log` 禁入仓」）。

### T-2 REF-01 拆分设计笔记（`crates/executor/src/expr/mod.rs`，5269 行，186 个 `fn`）
- **目标**：给出 ≥4 个子模块的划分，公开 API **零变更**；明确和 #4914 B1.3 的重叠区，并规定 B1.3 先合入。
- **非目标**：不改求值语义，不做性能优化。
- **影响面**：`sqlrustgo-executor` 的 `pub` 面；`src/engine_select.rs` 等调用方。
- **风险**：**High**（表达式求值是全部 SQL 语义的底座）。**回滚**：每片单 commit，`git revert` 即可。
- **验证（R2 执行时）**：`cargo fmt --check --all`；`cargo clippy --all-features -- -D warnings`；`cargo test -p sqlrustgo-executor --all-features`；`cargo test --test '*sql*' --all-features`；`bash scripts/gate/check_arch_invariants.sh`；`bash scripts/gate/check_anti_ignore_gate.sh`；`cargo public-api` 或 `rustdoc --output-format json` 前后 diff 为空（可证伪判据：公开 API 不变）。
- **证据**：`…/evidence/ref-01/<slice-n>/`。

### T-3 REF-02 拆分设计笔记（`stored_proc.rs`，4277 行）
- 同 T-2 结构。**额外约束**：排在 #4947（读路径 + `Self: Sized` 阻塞）之后，或者证明两者不重叠。**风险**：Med-High。
- 额外验证：存储过程、事务相关测试（`tests/integration/transaction/`），并确认 DML 生产路径（AGENTS §1.5）。

### T-4 REF-03 拆分设计笔记（`trigger.rs`，2627 行）
- 硬依赖 #4947 先落地（STAGE L911）。**风险**：Med。拆完目标是每个文件 < 1600。

### T-5 R3 C-ARCH-05 扩展提案
- **前置**：T-2..T-4 都已落地，并有 `check_arch_invariants.sh` NOTE 段**无 WARN** 的实测输出。
- **内容**：只出提案（diff + 影响评估），门禁语义修改**单独授权**；分母 168 不变（扩展是改既有 `check_arch_invariants.sh`，不新增 `check_*`）。
- **可证伪判据**：在拆分前的树上运行扩展后的门禁，**必须**报红（负向夹具），否则说明门禁是装饰。

### T-6（非 REF，阻断项）ignore 登记漂移 136/135
- 定位引入的提交，补登记或移除那个 `#[ignore]`。按 ADR-008 不得为了过绿而改登记上限。**风险**：Low。**需授权**：是（改 `tests/baseline/ignore_registry.json`）。

---

# 三、《Harness 治理启动包 v0.1》

## 3.1 Harness 启动检查单（可执行版）

| # | 步骤 | 命令 | 本地可跑 / CI-only | 本机实测 |
|---|---|---|---|---|
| H-1 | 身份锁定 | `git rev-parse HEAD; git branch --show-current; git status --short; git log -1 --pretty=fuller` | 本地 | ✅ |
| H-2 | 浅克隆检查 | `git rev-parse --is-shallow-repository`（为 true 时禁用 blame/作者结论） | 本地 | ✅ true |
| H-3 | 远端分层 | `git ls-remote <每个 remote> develop/v4.1.0 main base/v4.1.0-frozen`；252 不可达时标 UNKNOWN | 本地 | ✅（252 不可达） |
| H-4 | 阶段 | `sed -n '201p' docs/releases/v4.1.0/STAGE.yaml` | 本地 | ✅ ALPHA |
| H-5 | 工具链 | `command -v cargo rustc python3`；缺工具 ⇒ 依赖它的门禁一律 UNKNOWN，不计 FAIL/PASS | 本地 | ❌ cargo 缺失 |
| H-6 | D8 硬门禁 | `bash scripts/gate/check_arch_sem_debt.sh` | 本地 | 未跑（UNKNOWN） |
| H-7 | 架构不变量 | `bash scripts/gate/check_arch_invariants.sh` | 本地 | ✅ exit 0 |
| H-8 | 门禁编目 | `python3 scripts/gate/gate_registry.py check` | 本地 | ✅ fails=0 |
| H-9 | 诚实性 | `python3 audit/v4.1.0/honesty.py check` | 本地 | ✅ fails=0 |
| H-10 | 阶段清单 | `bash scripts/gate/check_stage.sh`（**会往 `docs/releases/v4.1.0/logs/` 写文件**，跑完要清理或在 worktree 里跑） | 本地，需 cargo | exit 1（见 1.5） |
| H-11 | 准入判据 | `python3 docs/refactor/v4.1.0-c/takeover/gen_verdict.py`（**没有 `--check`**；需要 `jsonschema==4.23.0`，本机是 4.10.3） | 本地 | 未跑 |
| H-12 | GitNexus | `gitnexus_impact` / `detect_changes`；索引不存在时降级并**在报告里声明降级** | 视环境 | ❌ 无索引 |

## 3.2 证据包规范（Evidence Bundle Spec）草案

```
docs/releases/v4.1.0/evidence/<topic>-<YYYYMMDD>-<seq>/
  MANIFEST.yaml        # 五字段 + 文件清单 + 每文件 sha256
  cmd-<nn>-<slug>.stdout
  cmd-<nn>-<slug>.stderr
  cmd-<nn>-<slug>.exit # 单行退出码
  env.yaml             # host / os / rustc / cargo / python / jsonschema 版本 / is_shallow
```

`MANIFEST.yaml` 字段：
```yaml
source_agent: <agent-id/model>
source_run: <topic>-<YYYYMMDD>-<seq>
timestamp: <ISO8601 UTC>
head: <40 位 sha>
base_sha: fd849ae20541901d5e59f45cd3ba2c51093f5d5e   # 内容基线，不随 tip 移动
tree_dirty: false
evidence_hash: sha256 of concatenated (path\tsha256\n) list, sorted
conflict_resolution: <与文档冲突时的取舍说明，或 "none">
files: [{path, sha256, cmd, exit}]
verdict_kind: MEASURED | ENV_UNKNOWN | DERIVED(<source_run>)   # 区分实测 / 环境缺失 / 转引
```

规则：
- 哈希用 `sha256sum`（Linux）或 `shasum -a 256`（macOS），结果相同。
- 禁止提交 `*.json` / `*.log` / `*.tbl`（与现行 `.gitignore` 一致）。
- **ENV_UNKNOWN 不得计入 FAIL 或 PASS**，这是从 1.5 的教训来的。
- 转引其他 Agent 的结论时，`verdict_kind: DERIVED(...)`，置信度降一级（AFP §5.4）。

## 3.3 基线对齐周报模板

```
## 基线周报 <YYYY-Www>
| ref | 252 | github | gitee | gitcode | 250 | 取样时间 |
| develop/v4.1.0 | | | | | | |
| main | | | | | | |
| base/v4.1.0-frozen | | | | | | |
BASE→HEAD 产品代码 diff：<空 / N 文件>
ALPHA 清单：MEASURED PASS x / FAIL y / ENV_UNKNOWN z（证据包路径）
准入判据：PASS/FAIL/WAIVED（gen_verdict 运行证据）
属主裁决位变化：<无 / 内容>
新增 UNKNOWN：
```

## 3.4 GitNexus / AOCI 合规检查点（PR 前强制）

| 检查点 | 对应 AGENTS 条款 | 不可用时 |
|---|---|---|
| 改符号前做 upstream impact | §4 MUST #1 | `rg` 调用方清单写进证据包，并标注 `degraded: gitnexus-missing` |
| commit 前 `detect_changes` | §4 MUST #2 | `git diff --stat` + 受影响 crate 列表 |
| HIGH/CRITICAL 风险告警 | §4 MUST #3 | 人工在 PR body 里写风险等级 |
| 重命名用 `gitnexus_rename` | §4 DO NOT | 禁止批量 find/replace；用 rust-analyzer rename |
| AOCI `aoci_maintain` / `update_entry` | §5 | 只读 `aoci.code.txt`，**不写**伪 receipt |

## 3.5 本方案条款 → 已有文档映射

| 方案条款 | 已有文档 | 缺口 / 最小补丁建议（未改） |
|---|---|---|
| 五字段证据 | `AGENTS.md` §9、`ANTI_FABRICATION_POLICY.md` §5.4、`ADR-014` | 缺统一的目录规范 → 3.2 |
| 区分「环境缺失」与「失败」 | `ADR-008`（SKIP/TIMEOUT 不计 PASS） | 没有明文规定「ENV_UNKNOWN 不计 FAIL」→ 建议在 ADR-008 补一条 |
| Phase 0 核验 | `AGENTS.md` §2 | 缺工具链检查（H-5）、浅克隆检查（H-2）、`check_stage.sh` 会写文件的提醒（H-10） |
| 基线定义 | `V410_BASELINE_FROZEN_DECLARATION.md`、`BASELINE_FREEZE.json` | AGENTS §12 的 BASE 和此一致 ✅ |
| 禁 GitHub / Gitea-only | `AGENTS.md` §3.2/§8 | 引用的 `docs/gitea-only.md` 不存在（T-4）→ 补文件或改引用 |
| 文档改动流程 | `DOC_CHECK_CORRECTION_RULES.md` | 本方案的所有落盘都要走它 |
| 版本口径 | `AGENTS.md:13`、`README.md:14` | 与 `Cargo.toml` 不符（T-1）→ 订正 |
| 复核命令可执行性 | `V410_REMOTE_BASELINE_ALIGNMENT_TASK.md` D11 | `--check` 不存在（T-5）→ 改成真实命令 |
| GitNexus 索引新鲜度 | `AGENTS.md` §4「已知问题」 | 在 H-12 要求声明降级 |

---

# 四、风险与待 Owner 裁决清单

| # | 裁决项 | 选项 | 建议 |
|---|---|---|---|
| O-A | 是否允许把本报告落盘到仓库（`docs/releases/v4.1.0/evidence/BASELINE_ALIGNMENT_2026-11-25.md`）并 push 到 **252** | (a) 允许，走 DOC_CHECK 5 步；(b) 只留本地 | (a)，但 push 只走 252（本机不可达 ⇒ 需要人工同步） |
| O-B | 是否把 `base/v4.1.0-frozen` 同步到 GitHub 镜像 | (a) 同步；(b) 保持不同步并在声明里注明 | 由属主决定；镜像写入属于 AGENTS §8 #11 的范围 |
| O-C | T-1 / T-4 / T-5 三处文档订正 | 逐项授权 | 授权，风险低 |
| O-D | ignore 漂移 136/135 的处置（T-6） | 补登记 / 移除 `#[ignore]` | 先定位引入提交，再决定 |
| O-E | 判据 #4（O1）、#6（Reviewer 名单）、O5 放行批次 | 见 `V410-OWNER-RULING.md` §4 | 不裁决就不能开 R2；R0/R1 可以先做 |
| O-F | 是否在本机装 Rust 工具链，补跑 7 道 ENV_UNKNOWN 门禁 | 装 / 不装 | 建议装，否则 1.5 的结论只能停在 UNKNOWN |

---

# 证据索引

| # | 命令 | 退出码 | 证据文件（`scratchpad/ev/`） | sha256 前缀 |
|---|---|---|---|---|
| E-1 | `git ls-remote`（github / gitee 各 3 个 ref） | 0 | 本文 1.2 节 | — |
| E-2 | `git fetch github develop/v4.1.0` + `merge --ff-only` | 0 | — | HEAD `3564a8b3` |
| E-3 | `sha256sum` 关键文件 | 0 | AGENTS `71b938c9…`；STAGE `5b150ca5…`；INDEX `f0900bfc…`；AFP `66f7fa2d…`；任务文档 `d700103b…`；STAGE_CONFIG `8bf4b67a…`；check_stage.sh `ecf3d694…` | — |
| E-4 | `bash scripts/gate/check_arch_invariants.sh` | 0 | `arch_inv.out` | `a42c3d49c41e76ae` |
| E-5 | `python3 scripts/gate/gate_registry.py check` | 0 | `gate_registry.out` | `3369b06db5d9c13d` |
| E-6 | `python3 audit/v4.1.0/honesty.py check` | 0 | `honesty.out` | `83fc651c16ef8e5e` |
| E-7 | `python3 …/gen_verdict.py --check` | 2 | `gen_verdict.out` | `1eaa0ec222301866` |
| E-8 | `bash scripts/gate/check_stage.sh` | 1 | `stage.out` / `gate_alpha_3564a8b_20261125_182442Z.log` | `ad205dcaf936fb57` |
| E-9 | anti-ignore 子门禁 | FAIL | `alpha_quality_Q3_ANTI_IGNORE_3564a8b_20261125_182442.log` | `6710a073a3d33482` |
| E-10 | `wc -l` 三个 REF 文件 + `execution_engine.rs` | 0 | 5269 / 4277 / 2627 / 1268 | — |
| E-11 | `git diff --stat fd849ae… HEAD -- crates/ src/` | 0 | 输出为空 | — |
