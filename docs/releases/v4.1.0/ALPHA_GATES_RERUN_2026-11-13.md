# ALPHA 门禁重跑与既往执行结果自查（2026-11-13）

> **文档性质**：进度记录 + 自查报告。**不是**阶段晋级结论，也**不是**门禁通过声明。
> **基线**：`develop/v4.1.0` @ `2792759137ef`（工作树干净，`git status --porcelain` = 0）
> **执行者**：`opencode-macmini`（OpenCode 线，接管会话）
> **阶段 SSOT**：`docs/releases/v4.1.0/STAGE.yaml` → `current_stage: "ALPHA"`
> **证据纪律**：文中每个数字都给出可复现命令；未跑完的部分写「进行中」，不预支结论。

---

## 0. 结论摘要

1. **A（ALPHA 10 道门禁全量重跑）已完成**：本地 00:35:29 起跑、01:01:46 结束（wall ≈26 分钟），**9 绿 / 1 红**。
   唯一红臂 `check_g1_tpch_baseline.sh` 已逐子检查归因为 **夹具缺失 + 门禁把 NOT-RUN 报成 mismatch**，
   并顺带查出它的另外 3 个子检查是**空转绿**（§6）。状态取 `待复核`（缺 `audit_run` manifest）。
2. **自查发现 4 项既往执行问题**，其中 2 项影响"结论可信度"，2 项影响"证据可追溯性"：
   - **S-1** 昨日「8 绿 / 2 红」的看板行与它引用的 run 目录**都不在 develop**（分居两条未合并分支，且证据提交发生在其自身 PR 合并之后）。
   - **S-2** 覆盖率门禁存在**同一 HEAD 两种相反读数**（87.70% vs 29.73%）；本轮第三次读数与 87.70% 逐位相同 ⇒ 29.73% 是离群值，原因指向共享 target 的测量面。
   - **S-3** `check_stage.sh` 在 ALPHA 阶段调用覆盖率臂时**没有声明 stage**，实际按 GA 的 85% 判，与脚本自身注释「每个调用点都显式声明 stage」不符。
   - **S-4** 树内存在**无 manifest 的门禁日志**（被 gitignore，不进版本控制），它们长得像证据但不能被引用。
3. 本会话自身也产生了 2 份未入库的运行日志（`gate_alpha_*.log`），已在 §4 显式记账，不当作证据。

---

## 1. A：本轮重跑的执行面

| 项 | 实测值 |
|---|---|
| 命令 | `bash scripts/gate/check_stage.sh --version v4.1.0` |
| 冻结提交 | `2792759137ef`（起跑前 HEAD，工作树干净） |
| 起跑（本地） | 2026-11-13 00:35:29 |
| 结束（本地） | 2026-11-13 01:01:46（wall ≈ **26 分钟**） |
| 运行日志 | `docs/releases/v4.1.0/logs/gate_alpha_2792759137_20261112_163529Z.log`（42 行，**未入库**） |
| 门禁清单 | 10 条（`--dry-run` 实测：7 个 `check_*` 脚本 + 3 个 cargo 命令） |
| 结果 | **9 绿 / 1 红**（`EXIT=1`） |
| 红臂 | `check_g1_tpch_baseline.sh` —— 归因见 §7，**不是 TPC-H 结果漂移** |
| 结论强度 | **待复核**：有逐臂 rc + 逐臂日志，但**没有 `audit_run` manifest / evidence_hash** ⇒ 按本行验收判据②，这些读数只能算"会话实测"，不能算归档证据 |

审计口径提醒：`check_stage.sh` 的 `LOG_DIR` 硬编码为 `docs/releases/${VERSION}/logs`，**没有环境变量可重定向**，
所以每跑一次都会在仓库树里留下日志文件。本轮已产生的未入库物见 §4。

---

## 2. 自查：既往执行结果是否有问题

### S-1　「8 绿 / 2 红」的结论不在 develop 上（影响：结论可信度）

**现象**：`docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md` 里能看到一行 `ALPHA-GATES-003`（声称完成、8 绿/2 红、run `20261112-alpha-gates-003`、`verify OK`）。

**实测**：

```
$ grep -c "ALPHA-GATES-003" docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md   # → 0
$ git show HEAD:docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md | grep -c ALPHA-GATES-003  # → 0
$ ls audit/v4.1.0/runs/ | grep alpha
20261107-alpha-gates-001
20261109-alpha-gates-002
20261111-alpha-gates-002
$ git merge-base --is-ancestor b5f2829e0c HEAD   # 看板行提交 → NOT-in-develop
$ git merge-base --is-ancestor e09cb5f98e HEAD   # run 目录提交 → NOT-in-develop
$ git branch -a --contains e09cb5f98e
+ evidence/v4.1.0-soak-ab003
$ git merge-base --is-ancestor eb6c4bdef0 e09cb5f98e && echo after-merge-point
after-merge-point
```

**判断**：
- 看板行在分支 `docs/board-ab-campaign-20261112`（`b5f2829e0c`、`77c4cf9e33`）；
- run 目录在分支 `evidence/v4.1.0-soak-ab003`（`e09cb5f98e`）；
- 而 `e09cb5f98e` 是**已被合并分支 tip `eb6c4bdef0` 的子提交** —— 即证据是在 PR #5766 合并**之后**才提交的，
  它**不会**随该 PR 进入 develop，必须另开 PR 才能落地。
- 结果：develop 上看板仍是 129 行、无该行；`docs/releases/v4.1.0/ALPHA_TO_BETA_PROMOTION_REVIEW_2026-11-12.md:215`
  却指向 `20261112-alpha-gates-003` —— 这是一个**悬空证据指针**（该处写的是"执行中"，属程度较轻的一档，但仍不可解析）。

**处置**：不在本会话擅自补写他人行的状态。本轮重跑用**新行 `ALPHA-GATES-004`** 记录，写清楚与 003 的关系，
避免两条线在同一行号上打架。

### S-2　覆盖率门禁出现两个相反读数（影响：结论可信度）

| 来源 | parser | planner | executor | transaction | storage | catalog | optimizer | types | 平均 | 判定 |
|---|---|---|---|---|---|---|---|---|---|---|
| 树内日志 `docs/releases/v4.1.0/logs/gates/check_coverage_v312.sh.log`（mtime 2026-11-12 02:33，**无 manifest**） | 86.00% | 88.71% | 82.79% | 93.29% | 80.80% | 87.33% | 87.64% | 95.04% | **87.70%** | PASS（85%） |
| 归档 run `20261112-alpha-gates-003`（有 manifest + sha256） | 9.51% | 1.21% | 12.90% | 2.29% | 37.19% | 8.74% | 71.00% | 95.04% | **29.73%** | FAIL（85%） |

**实测命令**：

```
$ cat docs/releases/v4.1.0/logs/gates/check_coverage_v312.sh.log        # 87.70% / measured 8/8 crates
$ git show e09cb5f98e:audit/v4.1.0/runs/20261112-alpha-gates-003/logs/004-alpha-gate-coverage-v312.stdout
$ git show e09cb5f98e:audit/v4.1.0/runs/20261112-alpha-gates-003/logs/004-alpha-gate-coverage-v312.stderr
FAIL: average 29.73% < threshold 85%
```

**判断（只到证据支持的程度）**：
- 两次都是 8/8 crate、同一门禁脚本（脚本最后一次改动 2026-10-24，早于两次测量），**同一 `types` 臂两次都是 95.04%**；
  其余 7 个 crate 差值 5–87 个百分点。
- 这种"一个 crate 完全一致、其余整体塌陷"的形态指向**测量面**（插桩二进制/`profdata` 与 target 不匹配），
  而非这些 crate 的代码在同一夜之间掉了 80 点覆盖率。
- 归档 run 自己的 `deviations` 就写着：工作树 target 指向共享 `CARGO_TARGET_DIR`（软链），并声称这样做"覆盖率也不会对着正在重建的 target 测量"。
  实测数字与该声称相反 ⇒ **该假设不成立**。
- **因此：87.70% 与 29.73% 不能同时成立，且倾向后者是环境异常**（本会话 §7 的第三次读数与 87.70% 完全一致，见下表）。
  但按本行验收判据②，87.70% 那一侧**仍缺 manifest/sha256/commit 绑定** ⇒ **可复现，但仍不可作为归档证据引用**。
- **待定位（下一步实验，不在本次声明范围）**：同一 HEAD 上跑两次同一条门禁——
  ① 默认 target；② 共享 target（软链）——比对逐 crate 数字。若 ② 复现塌陷，则根因在测量面，应修门禁而不是修代码。

**第三次读数（本会话可复核，仍无 manifest）**：`2792759137ef` 上 `check_coverage_v312.sh` 实测
`average coverage: 87.70% (threshold: 85%) — measured 8/8 crates`，逐 crate 与 11-12 02:33 那份**逐位相同**
（parser 86.00 / planner 88.71 / executor 82.80 / transaction 93.29 / storage 80.80 / catalog 87.33 / optimizer 87.64 / types 95.04）。
⇒ 三份读数里 **2 份一致、1 份是离群值**，离群值恰是那次把 target 指向共享目录的 run。

### S-3　ALPHA 阶段按 GA 阈值判覆盖率（影响：判据口径）

**实测**：

```
$ sed -n '85p' scripts/gate/check_coverage_v312.sh
COVERAGE_STAGE="${SQLRUSTGO_COVERAGE_STAGE:-ga}"
$ grep -rn "SQLRUSTGO_COVERAGE_STAGE" scripts/gate/check_stage.sh scripts/gate/check_alpha_v410.sh
（无输出）
$ grep -n "COVERAGE_MIN_ALPHA\|COVERAGE_MIN_BETA\|COVERAGE_MIN_RC\|COVERAGE_MIN_GA" docs/governance/STAGE_CONFIG.yaml
COVERAGE_MIN_ALPHA: 50 / COVERAGE_MIN_BETA: 75 / COVERAGE_MIN_RC: 80 / (GA 85)
```

**判断**：脚本注释明确写着「默认 GA 是刻意的，因为**每个调用点都显式声明 stage**」。
实测 `check_stage.sh` 与 `check_alpha_v410.sh` **都没有声明** ⇒ ALPHA 阶段的覆盖率臂实际按 **GA 85%** 判，
而 ALPHA SSOT 是 **50%**。这是门禁口径错配：一条 ALPHA 门禁用着比 ALPHA 更严的刻度，且判定文案会让人误读成"ALPHA 覆盖率不达标"。
（对本次两个读数而言结论不变：29.73% < 50 也红，87.70% > 85 也绿；错的是**判据标签**，不是这次的判定方向。）

**处置**：属门禁脚本域（`scripts/gate/`），本报告只登记，不擅自改判据阈值（改阈值属不可协商项）。

### S-4　树内的门禁日志长得像证据但不可引用（影响：可追溯性）

**实测**：

```
$ git ls-files docs/releases/v4.1.0/logs/gates/          # → 空（全部未跟踪）
$ git check-ignore -v docs/releases/v4.1.0/logs/gates/check_coverage_v312.sh.log
.gitignore:146:docs/releases/*/logs/*    ← 被忽略
$ git check-ignore -v docs/releases/v4.1.0/logs/gate_alpha_2792759137_20261112_163529Z.log
.gitignore:199:!docs/releases/*/logs/gate_*.log   ← 反向规则：运行日志是有意允许入库的
```

**判断**：
- 设计意图是「`gate_*.log` 可入库、`logs/gates/*.log` 不入库」，但**只有前者带 run 指纹（sha + UTC 时间戳）**；
  后者（逐臂日志）每次运行即被覆盖（本轮 00:35 就覆盖了 `check_alpha_v4.1.0.sh.log`）。
- 于是 S-2 表里那组 87.70% 的读数**只存在于一个会被下次运行覆写的文件里**，且无 manifest / 无 sha256 / 无 commit 绑定。
  它不能进任何结论。

**处置**：不改 `.gitignore`（文件治理属治理域）。改为在本报告里显式登记"哪份不能引用"。

---

## 3. 对看板的影响

- 新增行 **`ALPHA-GATES-004`**（本轮重跑 + 本自查），状态 **待复核**：9 绿/1 红已逐臂实测并归因，
  但缺 `audit_run` manifest ⇒ 不写"完成"。
- **不做**的事：不修改 `ALPHA-GATES-003` 所在分支的行文本；不把 `20261112-alpha-gates-003` 的结论搬进 develop 冒充本线结论；
  不给覆盖率写任何"通过/不通过"的总判定。

---

## 4. 本会话自身产生的未入库物（自我记账）

| 文件 | 来源 | 是否入库 | 处置 |
|---|---|---|---|
| `docs/releases/v4.1.0/logs/gate_alpha_2792759137_20261112_161634Z.log` | 本会话第一次尝试（120s 超时被 SIGTERM，仅 26 行、无任何臂结果）；归属依据 = mtime `2026-11-13 00:16:34` 与本会话首次超时时刻吻合 | 否 | **已删除**（空跑残留，留着会被误读成"跑过"） |
| `docs/releases/v4.1.0/logs/gate_alpha_2792759137_20261112_163529Z.log` | 本会话 A 的运行日志（增长中） | 否 | A 结束后按结果决定是否归档，不自动入库 |

说明：这两份文件的文件名带 UTC 时间戳，与本地时间差 8 小时，读日志时不要按本地时间反推。

---

## 5. 复现命令（全部只读）

```bash
# 阶段与基线
git rev-parse HEAD && git status --porcelain
grep -n 'current_stage' docs/releases/v4.1.0/STAGE.yaml

# S-1 证据是否在 develop
grep -c ALPHA-GATES-003 docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md
git merge-base --is-ancestor e09cb5f98e HEAD; echo $?   # 1 = 不在
git branch -a --contains e09cb5f98e

# S-2 两个覆盖率读数
cat docs/releases/v4.1.0/logs/gates/check_coverage_v312.sh.log
git show e09cb5f98e:audit/v4.1.0/runs/20261112-alpha-gates-003/logs/004-alpha-gate-coverage-v312.stderr

# S-3 阶段阈值
sed -n '85p' scripts/gate/check_coverage_v312.sh
grep -rn SQLRUSTGO_COVERAGE_STAGE scripts/gate/check_stage.sh scripts/gate/check_alpha_v410.sh

# S-4 日志是否被跟踪
git ls-files docs/releases/v4.1.0/logs/gates/ | wc -l
git check-ignore -v docs/releases/v4.1.0/logs/gates/check_coverage_v312.sh.log

# A 的最终结果
cat docs/releases/v4.1.0/logs/gate_alpha_2792759137_20261112_163529Z.log

# g1 归因：夹具是否在
ls -d ~/sqlrustgo-tpch/data /tmp/tpch-fixture 2>/dev/null
grep -n "TPCH_DATA_DIR" scripts/gate/check_g1_tpch_baseline.sh
# g1 的真实形态：NOT-RUN（不是 hash 不匹配）
cargo test --test tpch_hash_test -- --nocapture 2>&1 | grep -a "TPCH-NOT-RUN"
```

---

## 6. A 的最终结果与 g1 归因（本会话新增）

**结果：9 绿 / 1 红，`EXIT=1`。** 绿臂 —— `check_alpha_v4.1.0.sh`、`check_arch_invariants.sh`、
`check_arch3_no_bypass.sh`、`check_coverage_v312.sh`、`check_sql_corpus_gate.sh`、`check_anti_ignore_gate.sh`、
`cargo build --all-features`、`cargo test --all-features --lib`、`cargo fmt --check`。

**红臂 `check_g1_tpch_baseline.sh` 的归因（逐臂，不是"环境问题"）**：

该门禁 4 个子检查里，**3 个是空转绿，1 个把 NOT-RUN 报成了 mismatch**：

| 子检查 | 实测 | 性质 |
|---|---|---|
| 1/4 `tpch_gate_test` | PASS | 未核（本次未单独取证） |
| 2/4 `TPCH_FORCE=1 cargo test --test tpch_full_22_test` | `40 passed`，**finished in 0.03s** | **空转绿**：40 个测试 0.03 秒跑完 ⇒ 没有真跑 22 条查询 |
| 3/4 `cargo test --test tpch_hash_test` | `50 passed`，**finished in 0.04s** | **空转绿**：同上 |
| 4/4 `tpch_hash_compare.py --check <hash>` | `ERROR: cannot find hash in test output` → 门禁打印 `FAIL: python3 --check failed (hash mismatch)` | **假归因**：真实原因是 NOT-RUN，不是 mismatch |

**根因（两条，都可复现）**：

1. **夹具缺失**：`check_g1_tpch_baseline.sh` 只在 2/4 传了 `TPCH_FORCE=1`，**全脚本从未设置 `TPCH_DATA_DIR`**；
   本机 `~/sqlrustgo-tpch/data` 与 `/tmp/tpch-fixture` 都不存在。测试于是自报
   `TPCH-NOT-RUN: tpch_hash_test reason=fixture_missing detail=no TPC-H fixture found (set TPCH_DATA_DIR ...)`
   并直接返回 —— 但测试进程仍然 `ok`。
2. **比较器把 NOT-RUN 当成 mismatch**：`scripts/gate/tpch_hash_compare.py:36` 的 `HASH_RE` 只匹配
   `G1 PASS: TPC-H 22/22 baseline hash matches (<hex>)`；测试走 NOT-RUN 分支时不打印这行，
   于是 `parse_hash()` 调 `die("cannot find hash in test output", code=2)`，
   而 `check_g1_tpch_baseline.sh:141-149` 又把任意非零一律译成 `hash mismatch`。

**判断**：
- 这条红**不是** TPC-H 结果漂移，也不是 #5688 的数据集口径问题（数据集口径会走 `profile_mismatch` 分支，本次走的是 `fixture_missing`）。
- 更值得警惕的是它的**绿**：门禁在自己的 FAIL 原因之外，还把 3 个"什么都没执行"的子检查报成 PASS。
  这与看板 `FIX-TEST-HONESTY` 行（#5711「跳过的测试仍被计为通过」）是**同一类缺陷，发生在门禁路径上**，
  而那一行的修复（PR #5764）还没合并。
- 结论强度：以上全部是可复现的**会话实测**；因为逐臂日志在 `logs/gates/`（gitignore + 每次覆盖），
  按本行验收判据②，**不升级为归档证据**，状态取 `待复核`。

**修法建议（不在本次执行范围，需 owner/门禁域裁定）**：
① `check_g1_tpch_baseline.sh` 显式解析并校验 `TPCH_DATA_DIR`，缺失时判 **INCOMPLETE（三态）** 而不是让子检查报 PASS；
② `tpch_hash_compare.py` 区分 `NOT-RUN`（rc=2，文案写 "NOT-RUN: reason=..."）与真正的 hash mismatch（rc=1）；
③ 门禁不得把 rc=2 译成 "hash mismatch"。

---

## 7. 明确未做的事

- 未对覆盖率做任何"通过/不通过"总判定（S-2 的测量面未定案，且读数缺 manifest）。
- 未运行 S-2 提出的对照实验（共享 target vs 默认 target）—— 单侧即 >30 分钟，需单独排期。
- 未修 g1 的两条根因（改门禁脚本属门禁域，本报告只登记 + 给修法建议）。
- 未改动任何门禁脚本、阈值、`.gitignore`、分支保护。
- 未关闭/修改任何 issue；未触碰 `ALPHA-GATES-003` 所在分支。
