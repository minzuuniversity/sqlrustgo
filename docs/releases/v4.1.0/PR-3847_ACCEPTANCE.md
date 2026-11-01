# PR-3847 ACCEPTANCE — D6b 测试清单门禁静态化 + B1 最小真执行冒烟补偿

> **Issue**: #3847 · **分支**: `ci/3847-d6b-no-reexec` · **基线**: `57cef09f99`（`develop/v4.1.0`）
> **提交**: `19de541445`（D6b 静态化 + 冒烟门禁）+ 本次提交（B1 冒烟集 + 三态 CI 接线 + 文档集）
> **Version**: v4.1.0 · **Created**: 2026-11-01
> **Status**: 本机验收 PASS，待远程 CI 验证
> **配套文档**: `PR-3847_SPEC.md` / `PR-3847_TEST_PLAN.md` / `PR-3847_TEST_DESIGN.md` / `PR-3847_TEST_REVIEW.md`

---

## 1. 概述

`Hermes Pipeline / postcheck` 的 "D9 Full Gate Verification" 步骤里的 D6b 门禁，
把整个测试套件重跑了一遍（实测 1h44m48s、per-job ~130GB），与独立的 `test` job 完全重叠。

本 PR 做两件事：

1. **D6b 静态化**：`check_test_inventory.sh` 改为只做"清单一致性核对"（文件数 ↔ 声明数 ↔
   cargo metadata 数），秒级完成，磁盘归零。
2. **补最小真执行冒烟**（B1 裁定）：新增 `scripts/gate/check_d6_smoke.sh`，在 postcheck 内
   独立真跑 4 个硬冒烟目标 + 1 个条件目标，产出证据制品，退出码三态。

静态化会丢掉"测试真的 PASS"这一层证据，冒烟门禁正是为补这一层而存在。

## 2. 验收标准与结果

### AC-1：D6b 不再执行任何测试（二进制级保证）

**标准**：脚本内 `cargo` 子命令 ∈ {`metadata`, `--version`}；源码中不出现 `timeout` / `TIMEOUT`。

**验证**：`tests/integration/sql/test_inventory_gate_test.rs` 的
`test_d6_cargo_subcommand_whitelist` / `test_d6_never_executes_a_test_binary` /
`test_d6_has_no_timeout_wrapper` 三个用例。

**结果**：**PASS**（见 §3.1，12/12 通过）。

### AC-2：D6b 静态核对结论正确且秒级

**标准**：三方计数一致，产出证据制品，耗时秒级（原 1h44m48s）。

**验证**：

```bash
bash scripts/gate/check_test_inventory.sh
```

**结果**：**PASS**（见 §3.2）：479 测试文件 / 474 声明目标 / 474 cargo 目标 / 10 个共享模块 /
0 不一致；**耗时 13.8s**；证据写 `artifacts/gate/v3.8.0/d6_test_inventory.json`。

### AC-3：冒烟集规模受控且不含假绿/红目标

**标准**：硬集 ∈ [3,6]；不含 `tpch_gate_test`（fixture 缺失仍报 ok）；不含
`v312_13_load_data_sf1_test`（当前红，#3848）；条件目标单独成列。

**验证**：`test_d6_smoke_gate_is_minimal_real_execution` 的名单断言。

**结果**：**PASS**（见 §3.1）。硬集 4 个：`aggregate_smoke_test` / `wal_integration_test` /
`batch_insert_test` / `wire_protocol_smoke`；条件目标 1 个：`tpch_wire_smoke`。

### AC-4：冒烟硬目标在 base 上全部通过

**标准**：4 个硬目标在基线代码上均 PASS。

**验证**：`bash scripts/gate/check_d6_smoke.sh`（本机，基线 `57cef09f99` + 本 PR 改动）。

**结果**：**PASS**（见 §3.3）：4/4 PASS，耗时 23s/1s/4s/4s；条件目标在二进制齐备时 PASS（2s）。

### AC-5：二进制缺失 ⇒ DEGRADED（exit 2），不得静默 PASS

**标准**：移开 `sqlrustgo-mysql-server` 后重跑 ⇒ 退出码 2，制品记 `DEGRADED` +
`binary_not_found`，输出显式 DEGRADED 行。

**验证**：临时改名二进制后重跑门禁。

**结果**：**PASS**（见 §3.4）：`EXIT=2`，输出
`⚠️ D6 Smoke Gate: DEGRADED (4 passed, 1 degraded: tpch_wire_smoke)`，
制品 `{"target": "tpch_wire_smoke", "status": "DEGRADED", "reason": "binary_not_found"}`。

### AC-6：CI 步骤显式区分三态，FAIL 保持阻塞

**标准**：`ci.yml` 的 D6 Smoke 步骤读门禁自身退出码（`PIPESTATUS[0]`）；
0 ⇒ PASS 行；2 ⇒ DEGRADED 行且不阻塞；其余 ⇒ FAIL 行且 `exit`。

**验证**：`test_d6_smoke_gate_is_wired_into_postcheck` 断言 + YAML 解析。

**结果**：**PASS**（见 §3.5）：12/12 用例通过，`yaml.safe_load` 解析成功。

### AC-7：既有门禁不回归

**标准**：`check_full_gate_verification.sh` 三态逻辑不变；`check_test_manifest_honesty.sh` 结论口径不变。

**结果**：**PASS**（见 §3.2：manifest honesty gate exit 0，474 声明 / 0 孤儿测试文件）。

## 3. 验收证据（原文输出）

### 3.1 反回归测试

```
$ rustc --test --edition 2021 -o /tmp/inv_test2 tests/integration/sql/test_inventory_gate_test.rs
$ /tmp/inv_test2
running 12 tests
test test_d6_executable ... ok
test test_d6_delegates_to_manifest_honesty_gate ... ok
test test_d6_is_static_only_marker ... ok
test test_d6_reports_degraded_when_cargo_unavailable ... ok
test test_d6_collects_all_test_files ... ok
test test_d6_never_executes_a_test_binary ... ok
test test_d6_smoke_gate_is_minimal_real_execution ... ok
test test_d6_handles_subdir_tests ... ok
test test_d6_generates_evidence ... ok
test test_d6_has_no_timeout_wrapper ... ok
test test_d6_cargo_subcommand_whitelist ... ok
test test_d6_smoke_gate_is_wired_into_postcheck ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 3.2 D6b 静态核对

```
$ bash scripts/gate/check_test_inventory.sh
=== Step 2/3: cross-check against cargo metadata ===
  cargo reports 474 test targets, manifest declares 474
  PASS: C5: declared targets == cargo's test targets (474)

=== Step 3/3: verdict ===

  PASS: test manifest honesty gate: 474 declared targets, 0 orphan test files

=== D6 Test Inventory Summary (static-only) ===
Test files:              479
Declared test targets:   474
test targets reported by cargo: 474
Files without own [[test]] entry (shared modules): 10
Manifest honesty gate:   exit 0
Evidence written to artifacts/gate/v3.8.0/d6_test_inventory.json
✅ D6 Test Inventory: PASS (479 test files, 474 declared targets, 0 inconsistencies)

real	0m13.810s
```

### 3.3 冒烟门禁 —— PASS 路径（硬 4 + 条件 1）

```
$ CARGO_TARGET_DIR=/private/tmp/base6335/target bash scripts/gate/check_d6_smoke.sh
=== D6 Smoke Gate (minimal real execution, issue #3847) ===
Smoke targets: aggregate_smoke_test wal_integration_test batch_insert_test wire_protocol_smoke
Conditional targets: tpch_wire_smoke

  ✅ aggregate_smoke_test: PASS (23s)
  ✅ wal_integration_test: PASS (1s)
  ✅ batch_insert_test: PASS (4s)
  ✅ wire_protocol_smoke: PASS (4s)
  ✅ tpch_wire_smoke: PASS (2s)
✅ D6 Smoke Gate: PASS (5 target(s) passed)
```

### 3.4 冒烟门禁 —— DEGRADED 路径（二进制移开）

```
$ mv target/debug/sqlrustgo-mysql-server target/debug/sqlrustgo-mysql-server.bak
$ CARGO_TARGET_DIR=/private/tmp/base6335/target bash scripts/gate/check_d6_smoke.sh; echo "EXIT=$?"
=== D6 Smoke Gate (minimal real execution, issue #3847) ===
Smoke targets: aggregate_smoke_test wal_integration_test batch_insert_test wire_protocol_smoke
Conditional targets: tpch_wire_smoke

  ✅ aggregate_smoke_test: PASS (0s)
  ✅ wal_integration_test: PASS (1s)
  ✅ batch_insert_test: PASS (0s)
  ✅ wire_protocol_smoke: PASS (1s)
  ⚠️ tpch_wire_smoke: DEGRADED (canonical sqlrustgo-mysql-server binary not found;
      set SQLRUSTGO_MYSQL_SERVER_BIN or run 'cargo build -p sqlrustgo-mysql-server')
⚠️ D6 Smoke Gate: DEGRADED (4 passed, 1 degraded: tpch_wire_smoke)
   DEGRADED is not PASS: the conditional smoke targets did not run.
EXIT=2
$ mv target/debug/sqlrustgo-mysql-server.bak target/debug/sqlrustgo-mysql-server   # 已还原
```

制品 `artifacts/gate/v3.8.0/d6_smoke.json`（DEGRADED 态）：

```json
{
    "gate": "D6-smoke",
    "mode": "MINIMAL_REAL_EXECUTION",
    "targets": ["aggregate_smoke_test", "wal_integration_test", "batch_insert_test", "wire_protocol_smoke"],
    "conditional_targets": ["tpch_wire_smoke"],
    "passed": 4,
    "failed": 0,
    "degraded": 1,
    "results": [
        {"target": "aggregate_smoke_test", "status": "PASS"},
        {"target": "wal_integration_test", "status": "PASS"},
        {"target": "batch_insert_test", "status": "PASS"},
        {"target": "wire_protocol_smoke", "status": "PASS"},
        {"target": "tpch_wire_smoke", "status": "DEGRADED", "reason": "binary_not_found"}
    ],
    "result": "DEGRADED"
}
```

### 3.5 CI 工作流校验

```
$ python3 -c "import yaml; yaml.safe_load(open('.gitea/workflows/ci.yml'))"
YAML OK
```

D6 Smoke 步骤（`.gitea/workflows/ci.yml`）：

```yaml
        run: |
          set -uo pipefail
          timeout -k 30 3600 bash scripts/gate/check_d6_smoke.sh 2>&1 | tee d6_smoke.log
          rc=${PIPESTATUS[0]}
          case "$rc" in
            0) echo "D6 Smoke Gate: PASS" ;;
            2) echo "D6 Smoke Gate: DEGRADED (conditional smoke target did not run; not a PASS)" ;;
            *) echo "D6 Smoke Gate: FAIL (exit $rc)"; exit "$rc" ;;
          esac
```

## 4. 风险与遗留

| 编号 | 风险/遗留 | 影响 | 处置 |
|---|---|---|---|
| R1 | CI runner 上若无 `sqlrustgo-mysql-server` 二进制，`tpch_wire_smoke` 长期 DEGRADED | 该目标在 CI 上不提供证据 | 后续给 postcheck 增加 `cargo build -p sqlrustgo-mysql-server` 前置步骤；DEGRADED 已显式可见，不构成假绿 |
| R2 | `v312_13_load_data_sf1_test`（#3848）修复后未回接冒烟 | 真实 LOAD DATA 路径暂由 test job 覆盖 | 修复后作为条件目标回接 |
| R3 | 冒烟集规模断言 ∈ [3,6] | 将来扩容需同步改断言 | 由反回归测试强制，扩容需显式修改测试 |
| R4 | 硬目标全部为进程内测试，wire 路径依赖条件目标 | 若条件目标长期 DEGRADED，wire 覆盖依赖 test job | 见 R1 |

## 5. 验收结论

本机验收 **PASS**：AC-1 ~ AC-7 全部通过，证据见 §3 原文输出。
冒烟集采用 B1 裁定（硬 4 + 条件 1），DEGRADED 语义显式、不可被误读为 PASS。
同意开 PR，等待远程 CI 的 postcheck "D6 Smoke Gate (issue #3847)" 步骤验证。
