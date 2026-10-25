# D2 — CI 最小可信基线（`ci-trusted-baseline.yml`）· 2026-10-25

**依据**：Hub Issue #5 comment `198163`（Owner 执行指令，2026-10-25 14:48 CST）
**Owner 原文要求（D2）**：

> 新建 `ci-trusted-baseline.yml`：固定提交、fmt/clippy/check/test 全量、JUnit + `discovered/executed/ignored`
> 计数与最低期望数、独立 coverage（cargo-llvm-cov，不信自报）、Evidence Manifest（commit SHA + run ID +
> artifact SHA256）。负向注入：故意坏测试必须使流水线 FAIL；glob 无匹配必须 **INCONCLUSIVE**（不许 vacuous
> PASS）。出口判据：同一提交连续 3 次绿 + 1 次负向注入正确 FAIL + 独立 Judge 可复算。

**结论先行**：**测量逻辑已实跑并自证（23/23 断言 PASS）**；**流水线本身 NOT RUN**（CI 容量已死，见 §4），
因此**出口判据未达成**，本次交付是「可评审的第一版」，不是「已恢复的 CI」。

| 项 | 值 |
|---|---|
| 分支 | `mcode-macmini/ci-trusted-baseline`（base = `5d4817a06b` = `develop/v4.1.0` tip） |
| 交付文件 | 2 新增 + 1 行改动（见 §0） |
| 实跑证据 | `scripts/gate/trusted_baseline_verdict.py --selftest` → 23/23 PASS |
| 流水线状态 | **NOT RUN** —— CI 容量死（run `9992` 僵死 `queued`，全仓最新 run 活动停在 `2026-10-24T21:55:22Z`） |
| 出口判据 | **未达成**（无 run ⇒ 无「连续 3 绿」，无「负向注入正确 FAIL」） |
| 需要人工批准 | **是**（合并本 PR、改 CI 并发组/白名单、恢复 CI 容量） |

---

## 0. 交付物

| 文件 | 作用 |
|---|---|
| `.gitea/workflows/ci-trusted-baseline.yml` | 新流水线：固定提交 + fmt/clippy/check/test 全量 + JUnit + 计数 + 独立 coverage + Evidence Manifest + 三态判决 + 负向注入 |
| `scripts/gate/trusted_baseline_verdict.py` | 判决与证据的**确定性实现**：JUnit 生成、计数合并、Evidence Manifest、PASS/FAIL/INCONCLUSIVE 规则表。无网络、无模型调用、无时钟依赖 |
| `scripts/gate/check_ci_checkout_consistency.sh` | **一行改动**：把新 workflow 纳入 `GATED_WORKFLOWS`（否则该既有门禁会因「既非 gated 也非 exempt」而红） |

---

## 1. 为什么是**手动 dispatch**，而不是自动触发

这不是设计偏好，是三条硬约束的交集：

1. **不取消他人 CI。** 仓库并发组 `sqlrustgo-ci-pipeline-v2` 带 `cancel-in-progress: true`，任何自动触发的
   run 都会取消同一组里**其他 Agent** 的 run —— 协调规则明令禁止。本 workflow 使用**自己的**并发组
   `sqlrustgo-ci-trusted-baseline` 且 `cancel-in-progress: false`：既不取消别人，也不被别人的推送取消。
2. **「固定提交」只有在人指定提交时才有意义。** push 触发判定的是「推送带来的那个提交」，正是指令要摆脱的
   移动靶。
3. **钉住是可校验的，不是声明的。** 「Pin assertion」这一步把 checkout 结果与请求的 SHA 逐字比对，不一致
   直接硬失败（`PINNED_SHA_MISMATCH`），而不是安静地测了另一棵树。

**由此产生的一个必须说清的后果**：**本 workflow 不会自行产出任何判决**。判决只在有人针对一个具名提交手动
dispatch 时才存在。**没有判决 ≠ 判决为绿** —— 超时或被取消的 run 产出的是「无判决」。

---

## 2. 判决规则表（`scripts/gate/trusted_baseline_verdict.py`）

规则是**数据**不是控制流，一屏可读，且每条都有可测的分支：

| 规则 | 条件 | 结论 |
|---|---|---|
| `R1_EMPTY` | 声明的测试目标数为 0（glob 无匹配） | **INCONCLUSIVE** |
| `R2_COVERAGE` | 独立 coverage 工具不可用 | **INCONCLUSIVE** |
| `R3_COVERAGE` | 实测行覆盖率低于门槛 | FAIL |
| `R4_COUNT` | 实际执行数低于最低期望数 | FAIL |
| `R5_TESTFAIL` | 存在失败用例 | FAIL |
| `R6_NEGATIVE` | 负向注入模式但注入未造成失败 | FAIL |
| `R7_PASS` | 以上皆未触发 | PASS |

**R1 与 R2 就是 D2 的核心**：

- 既有流水线在 `cargo-llvm-cov` 缺失时打印 `NOT-RUN: reason=COVERAGE_TOOL_MISSING` 然后 `exit 0`
  —— 那是一个**贴着 NOT-RUN 标签的空过**。本实现把「工具缺失」记为 `status=missing`，判决落到
  **INCONCLUSIVE**。
- 「glob 无匹配」同理：声明集合为空时不是「0 个失败 = 通过」，而是**没测到东西**⇒ INCONCLUSIVE。

---

## 3. 已实跑（VERIFIED）

| 检查 | 命令 | rc | 结果 |
|---|---|---|---|
| 判决逻辑自检 | `python3 scripts/gate/trusted_baseline_verdict.py --selftest` | **0** | **23/23 断言 PASS**（含 R1–R7 每个分支、JUnit 确定性、计数、空日志=0 执行） |
| YAML 可解析 | `python3 -c "import yaml; yaml.safe_load(...)"` | 0 | 13 steps；`on = workflow_dispatch`；5 个输入；`concurrency.cancel-in-progress = false` |
| 门禁 C1 | `check_no_github_checkout` | 0 | OK：未使用 `actions/checkout` |
| 门禁 C2 | `check_canonical_bodies`（逐 workflow 单独跑） | 0 | OK：新 workflow 的 checkout 正文与 canonical **逐行一致** |
| 门禁 C3 | `check_input_overrides` | 0 | OK：未越权覆盖 `CI_DEPTH` / `CI_CHECKOUT_DIR` |
| 门禁 C4 | `check_exemptions` | 0 | OK：新 workflow 已列入 `GATED_WORKFLOWS`（附理由） |
| 门禁 C5 | `check_repo_scripts_after_checkout`（单独跑） | 0 | OK：没有任何步骤先于 checkout 调用仓库脚本 |

自检覆盖的 23 条断言里，最关键的四条是：`empty_glob_is_INCONCLUSIVE`、`coverage_tool_missing_is_INCONCLUSIVE`、
`negative_injection_must_fail`（负向注入**必须** FAIL）、`empty_log_executed_zero`（空日志的执行数必须是 0，
绝不能被当成通过）。

---

## 4. 未跑 / 未验证（NOT RUN / UNVERIFIED）

- **NOT RUN（环境所限，非未做）**：**整条流水线**。CI 容量已死 —— run `9992` 自 `2026-10-24T21:55:22Z` 起僵死在
  `queued`（4 个 job 从未调度），全仓最新 run 活动仍停在同一时刻，**无任何 id > 9993 的新 run**。
- **NOT RUN**：出口判据「同一提交连续 3 次绿」—— 一次 run 都没有。
- **NOT RUN**：出口判据「1 次负向注入正确 FAIL」—— 判决逻辑的 R6 分支已自检，但**未在真实 run 上**验证。
- **NOT RUN**：真实 run 上的独立 Judge 复算（判决逻辑已自证，但未绑定真实 run 的 SHA 与 run ID）。
- **UNVERIFIED**：容器 3h 上限下 `scope=all` 能否跑完。root 包测试在本 runner 上需要 86–147 min，成员包另计，
  两者相加大概率超限 ⇒ 计划用 `scope=root` 与 `scope=crates` 两次 dispatch 拼出全量基线。**这一点未经验证。**
- **NOT RUN**：`cargo fmt --all -- --check` / `clippy` / `check` 三步在真实提交上的结果。

---

## 5. 顺带发现的一个既有缺陷（与本次改动无关，但同属 D2 的病）

`scripts/gate/check_ci_checkout_consistency.sh` 在**未改动的** `develop/v4.1.0`（`5d4817a06b`）上**不终止**：
C1、C2 打印 `[OK]` 之后一直挂着。

- 对照实验：把 C1–C5 **逐项单独执行**，在 pristine base 与本分支上**全部通过**（含 C2 的正文比对与 C5 的
  步骤顺序检查，两者单独跑都在 1 秒内返回 rc=0）。
- ⇒ 卡点在**脚本的编排**，不在任何一条规则本身。
- 意义：一个**无法产出判决的门禁**，就是一套没有判决的测量系统 —— 正是 D2 要治的病。
- **本 PR 不修它**（修法需要 Owner 定夺，且会改动一条被多处引用的门禁）；建议 Owner 单独立项。

---

## 6. 复现命令

```bash
# 判决逻辑自检（不需要 cargo、不需要网络）
python3 scripts/gate/trusted_baseline_verdict.py --selftest

# 手动跑一次基线（需要 CI 容量恢复；未恢复前必然 NOT RUN）
#   在 Gitea UI 上对指向目标提交的 ref 触发 "CI Trusted Baseline"，
#   填 pinned_sha=<40hex>、mode=baseline、scope=root（再 scope=crates）
#
# 本地复算判决（把 run 产物放在一起即可）
python3 scripts/gate/trusted_baseline_verdict.py counts \
  --cargo-log cargo_test.log --gate-json execution_gate.json --out counts.json
python3 scripts/gate/trusted_baseline_verdict.py junit \
  --cargo-log cargo_test.log --out junit.xml
python3 scripts/gate/trusted_baseline_verdict.py verdict \
  --counts counts.json --coverage coverage.json \
  --commit <40hex> --run-id <run id> --mode baseline \
  --min-executed 0 --min-coverage-percent 0 \
  --manifest evidence_manifest.json \
  --artifact cargo_test.log --artifact junit.xml --artifact counts.json \
  --out verdict.json
# 退出码：0=PASS 1=FAIL 2=INCONCLUSIVE
```

---

**本文件不含任何密码、PAT、API Key、Cookie、私钥或凭据片段。**
