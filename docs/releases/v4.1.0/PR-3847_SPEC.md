# PR-3847 SPEC — D6b 测试清单门禁静态化 + 最小真执行冒烟补偿

> Issue: #3847 · 分支: `ci/3847-d6b-no-reexec` · 基线: `57cef09f99`（develop/v4.1.0）
> 提交: `19de541445` · 类型: Gate / CI 基础设施

---

## 1. 问题陈述

Issue #3847 记录的实况：`Hermes Pipeline / postcheck` 的第 27 步 "D9 Full Gate Verification"
在 D6b 阶段把**整个测试套件重新跑了一遍**，与独立的 `test` job 完全重叠。

根因在 `scripts/gate/check_test_inventory.sh`：它对每个 `tests/*.rs` 目标执行

```bash
timeout 180 cargo test --test "$name" -- --test-threads=1
```

实测后果（Gitea run 6408，job 13864 `postcheck`）：

| 指标 | 实测值 |
|---|---|
| D6b 单步耗时 | 1h44m48s（01:32:02Z → 03:16:48Z） |
| postcheck 单 job 耗时 | ~3h |
| runner per-job volume | ~130GB（曾把 250 根分区打到 97%） |
| 日志 | 被 Gitea 截断在 104887 字节，失败点不可见 |

两个根因必须分开处置：

- **R1（耗时/磁盘）**：D6b 的职责是"核对测试清单"，却做成了"重跑测试"。职责与代价错配。
- **R2（覆盖缺口）**：清单类门禁（`check_test_target_execution.sh`、`check_test_manifest_honesty.sh`）
  只能证明"某目标**被执行过**"，**不能证明"执行结果是 PASS"**。把 D6b 改成静态核对后，
  若不补真执行，整个 postcheck 就不再有任何"测试真的通过"的直接证据。

## 2. 范围

### 2.1 本 PR 做

1. `scripts/gate/check_test_inventory.sh` → **static-only**：
   - 枚举 `tests/**/*.rs` 实际测试文件；
   - 解析 `Cargo.toml` 的 `[[test]]` 段，解析 `name` ↔ `path` 映射；
   - `cargo metadata --no-deps`（只读，不编译）取 cargo 视角的目标数；
   - 委托既有权威门禁 `check_test_manifest_honesty.sh` 出最终结论；
   - 产出 `artifacts/gate/v3.8.0/d6_test_inventory.json`；
   - 退出码语义固定为 **0=PASS / 1=FAIL / 2=DEGRADED**（禁止静默 PASS）。
2. `scripts/gate/check_full_gate_verification.sh`：evidence 存在即 PASS；否则退化为静态 inventory；
   两者都失败才 DEGRADED（**不 exit 1**，避免把"门禁基础设施故障"误报成"测试失败"）。
3. **新增 `scripts/gate/check_d6_smoke.sh`**（补偿 R2）：postcheck 内独立真跑冒烟目标，
   每个 `timeout 900 cargo test --all-features --test <t> -- --test-threads=1`，
   产出 `artifacts/gate/v3.8.0/d6_smoke.json`，退出码 0/1/2。冒烟集分两类（裁定见 §3 D5）：
   - **硬目标（4 个，必须跑必须绿）**：`aggregate_smoke_test` / `wal_integration_test` /
     `batch_insert_test` / `wire_protocol_smoke`；
   - **条件目标（1 个）**：`tpch_wire_smoke`，依赖外部二进制
     `sqlrustgo-mysql-server`（可用 `SQLRUSTGO_MYSQL_SERVER_BIN` 指定）。
     二进制缺失 ⇒ 该目标记 **DEGRADED**（既非 PASS 也非 FAIL）；二进制存在但执行失败 ⇒ **FAIL**。
4. `.gitea/workflows/ci.yml`：在 postcheck 的 "D9 Full Gate Verification" 之后插入
   "D6 Smoke Gate (issue #3847)" 步骤，`if: always()`，外层 `timeout -k 30 3600`；
   步骤按门禁自身退出码（`PIPESTATUS[0]`）分支：0 ⇒ PASS、2 ⇒ 显式打印 DEGRADED 并放行、
   其余 ⇒ 打印 FAIL 并 `exit`（保持阻塞）。
5. 反回归测试 `tests/integration/sql/test_inventory_gate_test.rs`（12 个用例）。

### 2.2 本 PR 不做

- 不动 `test` job 的执行语义（那是 #3844 的范围）。
- 不修 v312_13 的 LOAD DATA 丢行缺陷（另案，见 #3848）。
- 不引入新的第三方依赖，不改动任何引擎源码。

## 3. 关键决策

| 编号 | 决策 | 理由 |
|---|---|---|
| D1 | D6b 静态化而非"复用 test job 制品" | 制品复用需要先改 `test` job 的产物格式，耦合面大；静态核对已能覆盖"清单一致性"这一职责本体 |
| D2 | 保留 `cargo metadata --no-deps` | 它是只读的、秒级的 cargo 视角真相；比解析 TOML 更不易漂移，且不触发编译 |
| D3 | 结论委托 `check_test_manifest_honesty.sh` | 不重复实现清单一致性判定，避免两套口径漂移 |
| D4 | 冒烟目标上限 6 个 / 下限 3 个 | 冒烟集必须"小"才有时间与磁盘意义；测试用反断言把这个规模钉死，防止将来又长成全量 |
| D5 | 冒烟集采用「硬 4 + 条件 1」（B1 裁定，DeepSeek DSQ31） | 硬集：`aggregate_smoke_test` / `wal_integration_test` / `batch_insert_test` / `wire_protocol_smoke`，覆盖进程内核心读写路径，base 上均 <25s 且稳定通过；`tpch_wire_smoke` 需要外部二进制，列为条件目标，二进制缺失判 DEGRADED 而非 FAIL |
| D6 | 退出码三态、禁止 `if: "false"` 式屏蔽 | DEGRADED 必须显式可见，否则等于把门禁悄悄关掉 |
| D7 | 冒烟集**不含** `tpch_gate_test` | 缺 fixture 时它打印 `TPCH-NOT-RUN: reason=fixture_missing` 却仍报 ok，是假绿 |
| D8 | 冒烟集**不含** `v312_13_load_data_sf1_test` | 该目标在目标分支上为红（#3848，346/630），红目标不能作为门禁目标；修复后再回接 |
| D9 | 不使用 `#[ignore]` 掩盖红测试 | `#[ignore]` 等于静默 PASS，与 D6 冲突 |

## 4. 对外接口

| 接口 | 形态 | 兼容性 |
|---|---|---|
| `artifacts/gate/v3.8.0/d6_test_inventory.json` | 新增制品 | 新增，向后兼容 |
| `artifacts/gate/v3.8.0/d6_smoke.json` | 新增制品 | 新增，向后兼容 |
| `scripts/gate/check_test_inventory.sh` | 行为变更：不再执行测试 | 消费方仅 postcheck，已同步 |
| `scripts/gate/check_d6_smoke.sh` | 新增脚本 | 新增 |

## 5. 影响面

- **时间**：postcheck 的 D6b 由 ~1h44m 降为秒级（实测 29.8s，见 ACCEPTANCE）。
- **磁盘**：postcheck 不再编译全部测试目标，per-job volume 从 ~130GB 降到常规量级。
- **正确性**：postcheck 恢复了"测试真的 PASS"的直接证据（冒烟），不因静态化而失守。
