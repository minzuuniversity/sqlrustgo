# PR-5933 ACCEPTANCE — 门禁执行证据

> **Issue**: #5592（看板 ISSUE）/ #5598（负向夹具台账）
> **Branch**: `fix/v410-criterion8-evidence-layer-20261126`
> **Base**: `a604ed9ab3`（`origin/develop/v4.1.0`）
> **Commits**: `47d590f5aa`（判据 #8 证据层修复）· `5324d2348c`（基线冻结声明 + 看板增量）·（归档与判据表重出）
> **Created**: 2026-11-26
> **Status**: 见 §5 —— **本 PR 的合并不解除判据 #4 与 #6**
> **配套文档**: `PR-5933_SPEC.md` / `PR-5933_TEST_PLAN.md` / `PR-5933_TEST_DESIGN.md` / `PR-5933_TEST_REVIEW.md`

---

## 1. 概述

本 PR 交付三件事：

1. **判据 #8 的证据层修复** —— 让负向夹具注入的红**可归因**：覆盖率门禁的基线红在**测量层**
   移除（**不排除测试**）、泳道**重排 + 无条件预热**、泳道**自报树身份**。
2. **基线冻结声明** —— 宣告 v4.1.0 **内容基线（BASE）冻结完成**，并把重构准入的现状按
   **映射层 / 签署层 / 证据层**三层写清。
3. **看板 4 行增量** —— `REF-BASE-1` / `REF-C8-1` / `REF-C4-1` / `REF-C6-1`。

**本 PR 不宣告重构准入的任何结论。** 12 条判据在本 PR 结束时是
**PASS 9 / FAIL 2 / WAIVED 1**（#8 转 PASS；#4 与 #6 仍在属主域）。

---

## 2. 门禁执行证据（逐条实测）

### 2.1 go/no-go 测量 —— 覆盖率门禁在完整树上**能测出 8 个 crate 的数字**

```bash
SQLRUSTGO_COVERAGE_STAGE=ga bash scripts/gate/check_coverage_v312.sh
```

**实测输出**：

```
Coverage criterion: stage=ga threshold=85%

excluded from coverage measurement (environment-dependent):
  file: crates/storage/tests/crash_commit_fsync_b1.rs
  file: crates/storage/tests/crash_wal_index_b1.rs
  file: crates/storage/tests/fault_coordination.rs
excluded from coverage measurement (perf-bound):
  file: crates/storage/tests/write_path_scaling_5181.rs
  file: crates/storage/tests/e2e_binlog_replication_test.rs
  file: crates/storage/tests/b22_concurrent_flush_ab.rs
excluded from coverage measurement (red-characterisation):
  file: crates/storage/tests/tf_t4_ai_candidate_v1.rs
  file: crates/storage/tests/tf_t4_stub_silent_success.rs
excluded test functions (23):
  committed_transaction_survives_sigkill_at_commit_fsync
  ... (共 23 个，含 8 个 tf_t4 函数)

Measuring coverage for sqlrustgo-parser ...
  sqlrustgo-parser: 86.00%
Measuring coverage for sqlrustgo-planner ...
  sqlrustgo-planner: 88.71%
Measuring coverage for sqlrustgo-executor ...
  sqlrustgo-executor: 82.80%
Measuring coverage for sqlrustgo-transaction ...
  sqlrustgo-transaction: 93.31%
Measuring coverage for sqlrustgo-storage ...
  sqlrustgo-storage: 80.93%          ← 这一行以前根本产不出来
Measuring coverage for sqlrustgo-catalog ...
  sqlrustgo-catalog: 87.33%
Measuring coverage for sqlrustgo-optimizer ...
  sqlrustgo-optimizer: 87.64%
Measuring coverage for sqlrustgo-types ...
  sqlrustgo-types: 95.04%
average coverage: 87.72% (threshold: 85%) — measured 8/8 crates
```

**rc=0**。

| 判据 | 期望 | 实测 | 结论 |
|---|---|---|---|
| 打印 **8 个** crate 的数字 | 8 | **8** | ✅ |
| `sqlrustgo-storage` 有值 | 有 | **80.93%** | ✅ |
| 不出现 `measured 7 of 8 crates` | 不出现 | **不出现** | ✅ |
| 不出现 `measured N of 8 crates` | 不出现 | **不出现** | ✅ |
| 完整树均值 **≥ 85**（go/no-go） | ≥85 | **87.72%** | ✅ **不触发「停」** |

> **go/no-go 通过** ⇒ 继续执行泳道。完整树均值 **< 85** 会**停**（不改判据、不声称放行、
> 按 SPEC §5 升级属主）—— 实测 87.72%，不触发。

### 2.2 R7 —— RED 特征测试**仍然红**（排除只作用于测量）

```bash
cargo test -p sqlrustgo-storage --test tf_t4_stub_silent_success --test tf_t4_ai_candidate_v1
```

**实测**：

```
test test_transaction_methods_reject_on_non_tx_backend ... FAILED
test test_delete_returns_real_count ... FAILED
test test_insert_then_scan_visible ... FAILED
test test_update_returns_real_count_and_persists ... FAILED
failures:
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out
```

**`git diff HEAD --stat -- crates/storage/tests/` 为空** ⇒ 源码**零改动**。
RED 特征测试仍 RED（#5913/#5914 仍挂 RED-LEDGER）。

### 2.3 门禁测试 —— 扩展已有断言，**不新增 `#[test]`**

```bash
cargo test --test alpha_gate_blocker_test --all-features
```

**实测**：

```
running 8 tests
test coverage_gate_runs_the_tests_serially ... ok
test coverage_gate_derives_features_from_the_crate_sources ... ok
test coverage_gate_excludes_the_environment_dependent_tests ... ok
test the_cargo_target_dir_dependency_is_runtime_resolved ... ok
test the_feature_gated_tests_compile_without_the_feature ... ok
test the_feature_gate_is_not_an_ignore ... ok
test coverage_gate_does_not_derive_features_for_a_plain_crate ... ok
test coverage_gate_derives_the_features_storage_actually_needs ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**测试计数仍是 8**（改动前也是 8）⇒ `tests/baseline/gate_test_baseline.json` 的计数**未受影响**。

### 2.4 门禁分母 / 诚实性 / 台账

```bash
$ python3 scripts/gate/gate_registry.py check
{"kind": "gate_registry_check", ..., "check_n": 168, "catalog_rows": 168,
 "by_class": {"stage_required": 34, "ci_reachable": 51, "orphan_referenced": 22, "dead": 61},
 "by_disposition": {"archive-candidate": 32, "keep-rewire": 12, "needs-execution-point": 25,
                    "needs-external-host": 12, "superseded": 2}, "fails": 0}
gate_registry: OK

$ python3 audit/v4.1.0/honesty.py check
{"kind": "honesty_check", ..., "gates": 168, "fixtures": 57, "fails": 0}
honesty: OK

$ python3 audit/v4.1.0/gen_neg_ledger.py --check
OK  .../NEG_FIXTURES_v4.1.0.jsonl
    已提交的台账与按当前参数重新生成的结果**逐字节一致**（57 行，sha256=507885f0…）
```

**分母仍为 168**（`check_n` 与 `catalog_rows` 两个数都是 168）。
`honesty.py emit` 之后 `with_catches` 从 17 变成 **18** —— 覆盖率门禁的
`effect.catches` 从 `[]` 变成 `["crate-coverage-below-threshold"]`。

### 2.5 看板

```bash
$ python3 scripts/gate/kanban_render.py check docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md
OK: KANBAN_v4.1.0_ALPHA.md ledger matches KANBAN_v4.1.0_ALPHA.jsonl (234 rows)     # rc=0

$ python3 scripts/gate/test_kanban_render.py
total=9 PASS=9 FAIL=0 INCOMPLETE=0

$ python3 scripts/gate/kanban_merge_policy.py docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.jsonl
{"problems": [], "rows": 234, "theirs_rows": 9, "verdict": "OK"}
```

### 2.6 其它

```bash
$ python3 scripts/gate/coverage_total_percent.py --selftest
coverage_total_percent selftest: 5/5 passed

$ bash -n scripts/gate/common.sh                      # OK
$ bash -n audit/v4.1.0/neg/verify_neg_fixtures.sh     # OK
$ bash -n audit/v4.1.0/neg/fx18-*.sh                  # OK
$ bash -n audit/v4.1.0/neg/fx15-*.sh                  # OK

$ python3 audit/v4.1.0/board_appendix_check.py
{"kind": "board_appendix_check", ..., "findings": 0}
OK: 看板附录与交接 ISSUE 均与仓内源文件逐字一致
```

---

## 3. 两层夹具泳道（**证据层修复的正面证明**）

### 3.1 泳道的自我报告

```
verify_neg_fixtures: scratch tree = /Volumes/workspace/dev/wt-v410-c8
verify_neg_fixtures: logs          = <run>/logs
verify_neg_fixtures: tree_sha=5324d2348c78e8fc4b9e8edb4fcece3f0359a6d1 dirty_files=0
verify_neg_fixtures: warming the target (this is unconditional; log: <run>/logs/warmup.log)
verify_neg_fixtures: warmup OK (rc=0)
```

**三项新机制全部生效**：树身份自报（含 `dirty_files=0`）· 无条件预热（`warmup OK`）。

### 3.2 逐门禁结果

| 门禁 | 夹具 | gate | stage |
|---|---|---|---|
| `scripts/gate/check_anti_ignore_gate.sh` | fx02 | **RED** | **RED** |
| `scripts/gate/check_g1_tpch_baseline.sh` | fx20 | **RED** | **RED** |
| `scripts/gate/check_sql_corpus_gate.sh` | fx19 | **RED** | **RED** |
| `scripts/gate/check_alpha_v410.sh` | fx15 | **RED** | **RED** |
| `scripts/gate/check_coverage_v312.sh` | fx18 | **RED** | **RED** |
| `scripts/gate/check_arch_invariants.sh` | fx16 | **RED** | **RED** |
| `scripts/gate/check_arch3_no_bypass.sh` | fx17 | **RED** | **RED** |

**最终汇总**：

```
verify_neg_fixtures: RED=7/7  INCONCLUSIVE=0/7
verify_neg_fixtures: 7/7 — every ALPHA gate is RED on both layers
```

**退出码 0**。**零** `exit_code=124`、**零** `INCONCLUSIVE`。

### 3.3 每条夹具日志的两层标记

每条夹具日志含：

```
FX-GATE-LAYER: gate=<gate> state=FAIL exit_code=1
FX-STAGE-LAYER: verdict=FAIL counts={...}
```

### 3.4 归档与可复核性

```
audit/v4.1.0/runs/20261126-criterion8-negfix-001/
  manifest.yaml
  logs/001-neg-fixtures-two-layer.stdout    （泳道汇总）
  logs/003-neg-fixtures.stdout              （台账 fx18 行的 evidence_log 指向它）
  logs/fx02-*.log … fx18-*.log              （逐夹具日志）
  logs/warmup.log                           （预热日志）
```

`audit_run.py verify --run-id 20261126-criterion8-negfix-001` ⇒ `verify OK`
（重新读取已提交的日志字节并重算 sha256，**不重新执行命令**）。

**文件类型约定**：`.stdout` / `.log` / `.yaml` —— 仓内**禁** `*.json` / `*.log`
（`check_no_log_tbl_json.sh` + `.gitignore`），因此逐夹具日志以 `.log` 后缀入库，
manifest 用 `.yaml`。

---

## 4. 判据表重出

```bash
python3 docs/refactor/v4.1.0-c/takeover/gen_verdict.py \
  --repo . \
  --neg-log audit/v4.1.0/runs/20261126-criterion8-negfix-001/logs/001-neg-fixtures-two-layer.stdout \
  --neg-run-date 2026-11-26 \
  --out docs/refactor/v4.1.0-c/takeover/V410-READINESS-VERDICT.json
```

| 项 | 前 | 后 |
|---|---|---|
| `summary.fail_criteria` | `[4, 6, 8]` | **`[4, 6]`** |
| `criteria[7].verdict` | `FAIL` | **`PASS`** |
| `criteria[7].observed` 里的树身份 | 「**推定/重构值**」（事后重构） | 「**观测值**」+ **逐字引用** `tree_sha=… dirty_files=0` 行 |

**不用 `--run-neg`**：它的内部超时是 4h，短于泳道的最坏耗时 ⇒ 会**静默超时**而不是给出判据。

---

## 5. 结论与边界

### 5.1 本 PR 达成的

| 项 | 结论 |
|---|---|
| 判据 #8 | **FAIL → PASS**（证据层修复 + 泳道 7/7 实跑） |
| 内容基线 | **冻结完成**（声明已发布，AFP 五字段齐备） |
| 看板 | 4 行增量，MD/JSONL 一致（rc=0），无单侧合并标记 |
| 门禁分母 | **168，未漂** |
| 阈值 | **未改** |
| `current_stage` | **未改**（仍 `ALPHA`） |

### 5.2 本 PR **没有**达成的（**明确否定**）

| 项 | 状态 | 责任方 |
|---|---|---|
| 判据 **#4**（映射层） | **仍 FAIL** | **属主裁决 O1**（执行者不得自拟映射，A2.2 R5 / O10） |
| 判据 **#6**（签署层） | **仍 FAIL** | **属主填 §3.1.1 名单表**（实施者不得自签，C7 / R-ORACLE） |
| 判据 12 的豁免背书 | `approved_by` **仍为空** | **属主写 §4.1 的一行** |
| 重构准入 | **未宣告** | 属主裁决位（§4）**仍为空** |
| 双血统咨询 | **不可得**（deepseek-web HTTP 401） | **属主恢复通道** |
| 13 条不变量覆盖 | **仍零覆盖** | 随批次指定责任包 |
| 核心重构包 | **未动** | 属主裁决 O5（放行批次） |

### 5.3 属主必须签字的动作（本 PR 引入）

> **覆盖率门禁在冻结基线上的判定变更** —— 从「不可测 ⇒ FAIL（`measured 7 of 8 crates`）」
> 变成「可测的真实数字（`measured 8/8 crates`，均值 87.72% vs GA 档 85）」。
>
> **这不是阈值放宽**：`COVERAGE_MIN_GA` 仍是 85；均值低于 85 仍然 FAIL。
> 已在**声明 §5.4、PR body、看板 §3 Q16 / §6** 三处显式列出。

---

## 6. 可复现命令清单

```bash
# 判据表
# 判据表（该生成器**没有 --check**；重跑到仓外再比对 summary）
python3 docs/refactor/v4.1.0-c/takeover/gen_verdict.py --out /tmp/verdict-check.json
python3 -c "import json;a=json.load(open('/tmp/verdict-check.json'));b=json.load(open('docs/refactor/v4.1.0-c/takeover/V410-READINESS-VERDICT.json'));print('MATCH:',a['summary']==b['summary'],a['summary'])"

# 门禁编目（分母 168）
python3 scripts/gate/gate_registry.py check

# 诚实性
python3 audit/v4.1.0/honesty.py check

# 负向夹具台账
python3 audit/v4.1.0/probe_neg_fixtures.py check

# 台账是生成物
python3 audit/v4.1.0/gen_neg_ledger.py --check

# 看板
python3 scripts/gate/kanban_render.py check docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md
python3 scripts/gate/test_kanban_render.py
python3 scripts/gate/kanban_merge_policy.py docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.jsonl

# 覆盖率门禁（完整树）
SQLRUSTGO_COVERAGE_STAGE=ga bash scripts/gate/check_coverage_v312.sh

# 两层夹具泳道
FX_LOG_DIR=<run>/logs bash audit/v4.1.0/neg/verify_neg_fixtures.sh

# 归档复核
python3 audit/v4.1.0/audit_run.py verify --run-id 20261126-criterion8-negfix-001

# 附录字节
python3 audit/v4.1.0/board_appendix_check.py

# 通用门禁
cargo clippy --all-features -- -D warnings
cargo fmt --check --all
cargo build --all-features
cargo test --test alpha_gate_blocker_test --all-features
```

---

## 7. 签署

| 角色 | 姓名 | 日期 | 状态 |
|---|---|---|---|
| 实施者 | `dsh-b` / Task 9 | 2026-11-26 | 已完成本 PR 的实现与取证 |
| **属主（门禁行为变更）** | — | — | **待签** |
| **属主（裁决位 §4）** | — | — | **待签** |
| **属主（O9 确认 §4.1）** | — | — | **待签** |
| **属主（Reviewer 名单 §3.1.1）** | — | — | **待填** |
