# PR-3847 TEST_DESIGN — 具体测试方法

> Issue: #3847 · 分支: `ci/3847-d6b-no-reexec` · 基线: `57cef09f99`

---

## 1. 反回归测试（源码级断言）

测试文件：`tests/integration/sql/test_inventory_gate_test.rs`（12 个用例）

| 用例 | 方法 |
|---|---|
| `test_d6_is_static_only_marker` | 守卫标记 `D6B_MODE: STATIC_ONLY_NO_CARGO` 恰出现 1 次 |
| `test_d6_cargo_subcommand_whitelist` | 解析脚本中全部 `cargo` 调用，子命令 ∈ {`metadata`, `--version`} |
| `test_d6_never_executes_a_test_binary` | 禁止 `cargo test` / `run` / `bench` / `nextest` / `miri` / `fuzz` |
| `test_d6_has_no_timeout_wrapper` | 源码中不出现 `timeout` / `TIMEOUT` |
| `test_d6_reports_degraded_when_cargo_unavailable` | cargo 不可用 ⇒ exit 2（DEGRADED），不得 exit 0 |
| `test_d6_generates_evidence` | 产出 `artifacts/gate/v3.8.0/d6_test_inventory.json` |
| `test_d6_delegates_to_manifest_honesty_gate` | 结论委托 `check_test_manifest_honesty.sh` |
| `test_d6_collects_all_test_files` / `test_d6_handles_subdir_tests` | 枚举覆盖 `tests/**` 子目录 |
| `test_d6_executable` | 脚本可执行且语法正确 |
| `test_d6_smoke_gate_is_minimal_real_execution` | 冒烟脚本含 `cargo test`，目标数 ∈ [3,6] |
| `test_d6_smoke_gate_is_wired_into_postcheck` | `ci.yml` 含 D6 Smoke 步骤且 `if: always()` |

运行方式（单文件、无需全量编译）：

```bash
rustc --test --edition 2021 -o /tmp/inv_test tests/integration/sql/test_inventory_gate_test.rs
/tmp/inv_test
```

## 2. 静态核对正确性

```bash
bash scripts/gate/check_test_inventory.sh
```

三方计数：实际测试文件数 / `Cargo.toml` `[[test]]` 声明数 / `cargo metadata --no-deps` 报告数。
结论由 `check_test_manifest_honesty.sh` 给出；制品写入 `artifacts/gate/v3.8.0/d6_test_inventory.json`。

## 3. 冒烟真执行

```bash
bash scripts/gate/check_d6_smoke.sh
```

**硬目标（4 个）**：`aggregate_smoke_test` / `wal_integration_test` / `batch_insert_test` /
`wire_protocol_smoke`，逐个 `timeout 900 cargo test --all-features --test <t> -- --test-threads=1`，
记录每个目标的通过/失败与耗时。

**条件目标（1 个）**：`tpch_wire_smoke`。先解析 `sqlrustgo-mysql-server` 二进制
（`SQLRUSTGO_MYSQL_SERVER_BIN` 优先，其次 `CARGO_TARGET_DIR/{debug,release}` 与 `target/{debug,release}`）：

| 二进制状态 | 该目标结论 | 门禁总退出码 |
|---|---|---|
| 存在且通过 | PASS | 0（若硬目标全绿） |
| 存在但执行失败 | **FAIL** | 1 |
| 缺失 | **DEGRADED**（`reason: binary_not_found`） | 2 |

制品写 `artifacts/gate/v3.8.0/d6_smoke.json`，总退出码 0=PASS / 1=FAIL / 2=DEGRADED。

CI 侧（`.gitea/workflows/ci.yml`）按门禁自身退出码分支，DEGRADED 必须打印显式
`D6 Smoke Gate: DEGRADED` 行且不得记为 PASS，FAIL 必须保持阻塞。

CI 中该脚本由 postcheck 步骤以 `timeout -k 30 3600` 包裹执行（外层保护，避免脚本自身
target 列表被改大时撑爆 job）。

## 4. 判伪标准

- 若某用例在"脚本被改回执行测试"时仍 PASS ⇒ 该用例无效，必须加强断言。
- 若冒烟目标集在 base 上有红 ⇒ 冒烟门禁本身不可信，先换目标或先修，不得以"CI 也红"为由放行。
- 若条件目标因二进制缺失而跳过 ⇒ 结论是 DEGRADED（exit 2），**不是** PASS；CI 步骤必须显式打印
  DEGRADED 行。二进制存在时的失败必须判 FAIL，不得降级为 DEGRADED 掩盖。
