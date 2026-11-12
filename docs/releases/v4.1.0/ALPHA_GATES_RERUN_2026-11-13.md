# ALPHA 门禁重跑与既往执行结果自查（2026-11-13）

> **文档性质**：进度记录 + 自查报告。**不是**阶段晋级结论，也**不是**门禁通过声明。
> **基线**：`develop/v4.1.0` @ `2792759137ef`（工作树干净，`git status --porcelain` = 0）
> **执行者**：`opencode-macmini`（OpenCode 线，接管会话）
> **阶段 SSOT**：`docs/releases/v4.1.0/STAGE.yaml` → `current_stage: "ALPHA"`
> **证据纪律**：文中每个数字都给出可复现命令；未跑完的部分写「进行中」，不预支结论。

---

## 0. 结论摘要

1. **A（ALPHA 10 道门禁全量重跑）已完成并归档**：本地 00:35:29 起跑、01:01:46 结束（wall ≈26 分钟），**9 绿 / 1 红**。
   归档 run `20261113-alpha-gates-004`（11 臂 = 清单 10 条 + 1 条 stage=alpha 对照臂，`verify OK`，
   `evidence_hash=sha256:94e1d412…`）+ 补测 run `20261113-alpha-gates-004b`（聚合门禁臂，
   `verify OK`，`sha256:a9b7f714…`）。004 的 001 臂 FAIL(127) 是**操作者把文件名写错**
   （`check_alpha_v4.1.0.sh` ≠ `check_alpha_v410.sh`），记录不篡改、由 004b 补测。
   红臂 g1 的缺陷已由 PR #5773 修复并合并（三态：缺夹具 ⇒ INCOMPLETE）。
2. **自查发现 4 项既往执行问题**，其中 2 项影响"结论可信度"，2 项影响"证据可追溯性"：
   - **S-1** 昨日「8 绿 / 2 红」的看板行与它引用的 run 目录**都不在 develop**（分居两条未合并分支，且证据提交发生在其自身 PR 合并之后）。
   - **S-2** 覆盖率门禁出现**同一 HEAD 两种相反读数**（87.70% vs 29.73%）——归档后三读数 3:1（本 run 的 GA 臂 + ALPHA 对照臂 + 11-12 树内日志同为 87.70%），29.73% 确认为共享 target 的测量面离群值。
   - **S-3** `check_stage.sh` 在 ALPHA 阶段调用覆盖率臂时**没有声明 stage**，实际按 GA 的 85% 判——本 run 的对照臂已把"ALPHA 50% 阈值下同样 87.70% PASS"变成归档数据。
   - **S-4** 树内存在**无 manifest 的门禁日志**（被 gitignore，不进版本控制），它们长得像证据但不能被引用。
3. **空转绿是家族不是孤例**（§8）：`check_g1_tpch_baseline.sh`（已修，#5773）、`check_p35_simd.sh`（动态实锤）、
   `g12_sysbench / g16_compatibility / p22_time_travel / p32_statistics / p33_cost_optimizer / p34_parallel_executor`
   （逐字节同一模板）。其中 p22 是 RC+GA required、g16 是 GA required —— 这两道阶段门禁在无夹具宿主上
   从未真正验过 TPC-H 却报 PASS。
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
| 结果 | **9 绿 / 1 红**（`EXIT=1`）；后续已归档：run `20261113-alpha-gates-004`（11 臂，`verify OK`）+ `20261113-alpha-gates-004b`（聚合门禁臂补测，`verify OK`） |
| 红臂 | `check_g1_tpch_baseline.sh` —— 归因见 §6，**不是 TPC-H 结果漂移**；缺陷已由 PR #5773 修复合并 |
| 结论强度 | **已升级为归档证据**：`audit_run.py verify` 双 run OK，`evidence_hash` 见 §0；唯一披露：004 的 001 臂 FAIL(127) 是操作者文件名笔误，由 004b 补测 |

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

**第三次读数（已随归档升级）**：`2792759137ef` 上 `check_coverage_v312.sh` 实测
`average coverage: 87.70% (threshold: 85%) — measured 8/8 crates`，逐 crate 与 11-12 02:33 那份**逐位相同**
（parser 86.00 / planner 88.71 / executor 82.80 / transaction 93.29 / storage 80.80 / catalog 87.33 / optimizer 87.64 / types 95.04）；
且归档 run `20261113-alpha-gates-004` 的对照臂（`SQLRUSTGO_COVERAGE_STAGE=alpha`，阈值 50%）给出**同一个 87.70%**。
⇒ 归档读数 2 份 + 树内日志 1 份 = 3:1，离群值正是把 target 指向共享目录的那次。
判据②（run id + evidence_hash）对归档的 2 份已满足；树内那份仍不可引用。

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

### S-3 的修复（2026-11-13，随本 PR）

`scripts/gate/check_stage.sh` 现在对覆盖率臂显式声明被判定阶段
（`SQLRUSTGO_COVERAGE_STAGE=<stage>`，dry-run 可见、exec 时导出；TDD 夹具
`scripts/gate/test_coverage_stage_declaration.py`：RED 2 失败 → GREEN 4/4 → 反向变异翻红还原复绿）。
缺省仍是 `ga`（#5581 的 fail-closed 设计不动）；改的只是"知道自己阶段的调用方必须声明"。
归档对照臂（run 004 的 005 号臂）证明 ALPHA(50%) 下同为 87.70% PASS ⇒ 本修复只改标签，不改判定方向。

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

- **`ALPHA-GATES-004` 已升级为完成**：run `20261113-alpha-gates-004`（11 臂）+ `004b`（聚合门禁臂补测）
  双 `verify OK`，evidence_hash 见 §0；001 臂的操作者笔误如实披露在行内。
- **新增行 `VACUOUS-GREEN-1`**（空转绿家族）：p35 动态实锤 + 6 道逐字节同模板 + 9 道静态候选；
  修复方案已备（共用实现 + 七调用方），另开 PR。
- **不做**的事：不修改 `ALPHA-GATES-003` 所在分支的行文本；不把 `20261112-alpha-gates-003` 的结论搬进 develop 冒充本线结论；
  不给覆盖率写任何"通过/不通过"的总判定。

---

## 4. 本会话自身产生的未入库物（自我记账）

| 文件 | 来源 | 是否入库 | 处置 |
|---|---|---|---|
| `docs/releases/v4.1.0/logs/gate_alpha_2792759137_20261112_161634Z.log` | 本会话第一次尝试（120s 超时被 SIGTERM，仅 26 行、无任何臂结果）；归属依据 = mtime `2026-11-13 00:16:34` 与本会话首次超时时刻吻合 | 否 | **已删除**（空跑残留，留着会被误读成"跑过"） |
| `docs/releases/v4.1.0/logs/gate_alpha_2792759137_20261112_163529Z.log` | 本会话 A 的运行日志 | **是**（PR #5772 已入库；`logs/gate_*.log` 在文件治理允许清单内） | 已作为 A 的运行日志引用 |

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

**修法（已执行）**：PR #5773 已按上述三条修复合并进 develop（`71114a717e`，commit `88f1471624`）：
夹具 preflight + 三态 + 比较器四态分类；TDD RED→GREEN + 2 臂反向变异。
本 run 的 008 臂留的是**修复前**的原始现场（manifest 不可改，这正是审计轨迹的价值）。

---

## 8. 空转绿家族扫查（2026-11-13，建议③的执行记录）

**方法**：静态扫 `scripts/gate/*.sh` 里引用夹具门控测试套件（tpch / soak / diag / oracle）的门禁，
对每个候选查两点——①是否设置 `TPCH_DATA_DIR`；②是否对 `NOT-RUN` / 实际执行量有任何感知。
静态命中的候选用**动态探针**逐个证实，只把动过手的写成结论。

### V-1　`check_p35_simd.sh`（动态实锤）

```
$ ls -d ~/sqlrustgo-tpch/data /tmp/tpch-fixture 2>/dev/null   # 两者都不存在
$ cargo test --test tpch_gate_test -- --nocapture 2>&1 | grep -aE "TPCH-NOT-RUN|test result"
TPCH-NOT-RUN: tpch_gate_test reason=fixture_missing detail=no TPC-H fixture found (set TPCH_DATA_DIR to a directory with the 8 .tbl files)
test result: ok. 40 passed; 0 failed; ... finished in 0.03s
$ bash scripts/gate/check_p35_simd.sh
  [7/7] ✅ PASS: TPC-H gate (22/22) maintained
=== G10 Gate (P3-5): PASS ===
rc=0
```

根因：`check_p35_simd.sh:103` 用 `grep -E "test result.*ok"` 判定——测试缺夹具时自报 NOT-RUN
后仍以 `test result: ok` 返回（一条查询没跑），grep 只看 "ok" ⇒ 子检查判 PASS，文案声称 "TPC-H gate (22/22) maintained"。

### V-2　同模板共 6 道（逐字节同一片段，`grep -B1 -A3` 实测）

`check_g12_sysbench.sh:88`、`check_g16_compatibility.sh:116`、`check_p22_time_travel.sh:102`、
`check_p32_statistics.sh:109`、`check_p33_cost_optimizer.sh:102`、`check_p34_parallel_executor.sh:103`
—— 六处均为
`TPCH_PASSED=$(cargo test --test tpch_gate_test 2>&1 | grep -E "test result.*ok" | head -1)`
+ `[N/N] ✅ PASS: TPC-H gate (22/22) maintained`。

### V-3　阶段影响（STAGE_CONFIG 实测）

`check_p22_time_travel.sh` 是 **RC 与 GA 的 required 门禁**；`check_g16_compatibility.sh` 是 **GA required**；
`check_g12_sysbench.sh` / `check_g11_qps.sh` 是 optional。⇒ **RC/GA 的 required 门禁在无夹具宿主上
从未真正验过 TPC-H，却计入"已绿"**。p32/p33/p34/p35 不在 STAGE_CONFIG 清单（dev/质量门禁）。

### V-4　对照组（mild）：g11 / g13 / g14

`check_g11_qps.sh:75` / `check_g13_stability.sh:87` / `check_g14_real_crash.sh:74` 也跑 `tpch_gate_test`，
但只把它当 **WARN 级 drift check**（exit≠0 才置 TPCH_PASSED=""），不把它写成 PASS 声明 —— 危害级别低一档，
但同样不感知 NOT-RUN（fixture_missing 时 rc=0 ⇒ 无 WARN），一并列入待办。

### 修复方案（已执行，2026-11-13）

- 新增 `scripts/gate/tpch_gate_subcheck.sh`：三态 0=PASS / 2=INCOMPLETE / 1=FAIL；
  缺夹具或测试自报 NOT-RUN ⇒ INCOMPLETE，且 **preflight 先行、不执行任何 cargo 调用**（秒级返回）；
- 7 道门禁的子检查改为调用它（同一次修改里还消灭了旧 else 分支的
  `[N/N] ✅ PASS (warned): TPC-H gate check skipped`——失败分支自己也在报 PASS），
  每道门禁末尾的 `exit 0` 改为按 `TPCH_RC` 三态收口（PASS / INCOMPLETE+exit 2 / FAIL+exit 1）；
- TDD：`scripts/gate/test_tpch_gate_subcheck_honesty.py`（A 侧钉共用实现三态 + UNDISCLOSED 不得报 PASS；
  B 侧以 p35 端到端钉调用方）。**RED**（先只有夹具）：6 条断言失败（127 / 旧门禁 rc=0 且声称 maintained）；
  **GREEN**（实现 + 七调用方）：10/10 通过；
- **反向变异**：把共用实现的夹具检查与 NOT-RUN 感知同时禁用（模拟旧的 grep-only 行为）
  ⇒ 夹具 exit=1，4 条断言翻红；还原后 10/10 复绿；
- **端到端 7/7**（`TPCH_DATA_DIR=/nonexistent` 实跑）：p35 已含在夹具 B 侧；其余 6 道
  `g12_sysbench / g16_compatibility / p22_time_travel / p32_statistics / p33_cost_optimizer / p34_parallel_executor`
  实测 **rc=2 且输出含 INCOMPLETE**（原先全部 rc=0 + 声称 maintained）。

### 静态候选的动态证实结果（2026-11-13，两个都动了手）

**① `check_g_correctness_v390.sh` ⇒ 清白（fail-closed，判红方向正确）**：

```
$ TPCH_DATA_DIR=/nonexistent-tpch-fixture-for-test bash scripts/gate/check_g_correctness_v390.sh
test result: ok. 40 passed ... finished in 0.05s      ← tpch 套件同样是 NOT-RUN 空转
[2/3] Results: 0 PASS, 0 ERR (out of 0)
  FAIL: 0 < 6
=== Gate-C: FAIL ===   (rc=1)
```
它**数的是真实执行量**（0 PASS out of 0 ⇒ FAIL），没有让 `test result: ok` 骗过 ——
缺夹具时它判红，红的原因是对的（"0 < 6"）。不是空转绿。

**② `check_p13_soak_test.sh` ⇒ 新发现（轻一档：幻影证据，不是空转绿）**：

```
$ TPCH_DATA_DIR=/nonexistent-tpch-fixture-for-test bash scripts/gate/check_p13_soak_test.sh
  [5/9] ✅ PASS: soak_test 10 passed (≥10)
=== G7 Gate: PASS ===   (rc=0)
$ cargo test --test soak_test -- --nocapture
test result: ok. 10 passed ... finished in 0.00s       ← 10 个测试 0.00s
```
其测试底座 `soak_test_harness.rs:89 run_soak_smoke` 是**模拟查询环**（源码注释原文
`// Simulated query loop`，时延 `0.5 + (i % 3) * 0.5` 是造出来的，内存增长是每 100 次加 1KB 的常量）。
⇒ `[5/9] soak_test 10 passed` 验的是**harness 计分算术**，不是任何一次真实 soak；
而 `=== G7 Gate: PASS ===` 这个字样容易被读成"soak 有证据"。真正的 168h SOAK 属 `ING-1`（从未闭合）。
该发现记在看板行 `VACUOUS-GREEN-1`（轻档：测试名带 "smoke"、代码有披露，但门禁输出未声明"模拟"）。

### 静态候选（剩余未证实的）

无 —— 原队列里的两个已全部动态证实（一清一轻档发现）。

---

## 9. 明确未做的事（随执行进度更新）

- ~~未对覆盖率做任何"通过/不通过"总判定~~ → **已做**：ALPHA 阈值（50%）下 87.70% PASS（归档对照臂）。
- ~~未运行 S-2 提出的对照实验~~ → **已做**：同一次测量里 GA(85%) 与 ALPHA(50%) 双读数（run 004 的 004/005 臂）。
- ~~未修 g1 的两条根因~~ → **已修**：PR #5773 已合并。
- ~~未修空转绿家族其余 7 道~~ → **已修**：共用实现 + 七调用方 + TDD 夹具，见 §8（本 PR）。
- ~~S-3 的口径修复~~ → **已修**：`check_stage.sh` 覆盖率臂显式声明阶段（见 §2 S-3）。
- ~~静态候选证实~~ → **已做**：`check_g_correctness_v390.sh` 清白（数执行量，判红方向正确）；`check_p13_soak_test.sh` 轻档发现（模拟查询环，见 §8）。
- **未做**：`check_p13_soak_test.sh` 输出补一句"模拟 harness smoke，非真实 soak"（轻档，另开 PR 或并入下次门禁改动）。
- **未实测**：g1 / subcheck 在**夹具存在**时的 PASS 路径（本机无 TPC-H 数据；`classify()` 单测覆盖该分支，不做超出证据的声明）。
- 未改动任何门禁阈值、`.gitignore`、分支保护。
- 未关闭/修改任何 issue；未触碰 `ALPHA-GATES-003` 所在分支。
