# PR-3847 TEST REVIEW — D6b 门禁静态化 + B1 冒烟补偿的独立测试审核

> **Issue**: #3847 · **分支**: `ci/3847-d6b-no-reexec` · **基线**: `57cef09f99`（`develop/v4.1.0`）
> **审核日期**: 2026-11-01 · **审核者**: 独立验证方（非本变更作者）
> **worktree**: `/private/tmp/wt-3847`
> **Source PR-SPEC**: `docs/releases/v4.1.0/PR-3847_SPEC.md`
> **Source PR-TEST_PLAN**: `docs/releases/v4.1.0/PR-3847_TEST_PLAN.md`
> **Source PR-TEST_DESIGN**: `docs/releases/v4.1.0/PR-3847_TEST_DESIGN.md`

---

## 1. 审核范围与基线

### 1.1 本次审核覆盖的变更

| 路径 | 类别 | 说明 |
|---|---|---|
| `scripts/gate/check_test_inventory.sh` | 修改 | D6b 改为静态一致性核对，不再执行测试 |
| `scripts/gate/check_full_gate_verification.sh` | 修改 | evidence 存在即 PASS，否则退化为静态 inventory |
| `scripts/gate/check_d6_smoke.sh` | 新增 | 最小真执行冒烟门禁（硬 4 + 条件 1） |
| `.gitea/workflows/ci.yml` | 修改 | postcheck 新增 "D6 Smoke Gate (issue #3847)" 步骤，按退出码三态分支 |
| `tests/integration/sql/test_inventory_gate_test.rs` | 修改 | 反回归测试 12 例，追加 B1 冒烟集语义断言 |
| `docs/releases/v4.1.0/PR-3847_{SPEC,TEST_PLAN,TEST_DESIGN,TEST_REVIEW,ACCEPTANCE}.md` | 新增 | 本 PR 文档集 |

### 1.2 不在审核范围

- `test` job 的执行语义（#3844 范围，暂缓合并）。
- `v312_13_load_data_sf1_test` 的 LOAD DATA 丢行缺陷（#3848）。
- 其余 474 个测试目标的通过性（由 `test` job 负责）。

## 2. 审核判定摘要

| 审核项 | 判定 | 依据 |
|---|---|---|
| D6b 已静态化（不再执行测试） | **PASS** | 反回归测试 12/12；`cargo metadata` 白名单断言 |
| D6b 三方计数一致 | **PASS** | 479 文件 / 474 声明 / 474 cargo / 0 不一致 |
| 冒烟集规模受控 | **PASS** | 硬集 4 个，反断言钉死 ∈ [3,6] |
| 冒烟集不含假绿/红目标 | **PASS** | 排除 `tpch_gate_test`（fixture 缺失仍报 ok）与 `v312_13`（当前红） |
| 条件目标 DEGRADED 非静默 PASS | **PASS** | 二进制移开实测 exit 2 + 制品 `binary_not_found` |
| 条件目标失败判 FAIL | **PASS**（源码审阅） | 二进制存在时走 `timeout … cargo test` 失败分支 → `FAILED` → exit 1 |
| CI 步骤三态显式 | **PASS** | `PIPESTATUS[0]` 分支；DEGRADED/FAIL 各有显式行；FAIL 保持 `exit` |
| 回归测试有效（非恒真） | **PASS** | 断言命中具体标记/名单，篡改冒烟集即失败 |

**审核结论：PASS，同意开 PR。** 冒烟门禁本身不引入假绿；它补上了静态化后唯一缺失的"执行真的 PASS"证据。

## 3. 关键审核点

### 3.1 为什么 `tpch_wire_smoke` 只能是条件目标

`tpch_wire_smoke` 会 spawn 规范二进制 `sqlrustgo-mysql-server`。二进制缺失时测试自身
`panic!("canonical binary not found")`，若把"二进制缺失"判成 FAIL，则 CI 环境差异会被误报成
产品缺陷；若判成 PASS，则是假绿。本 PR 的处理是**单独一类：DEGRADED + exit 2**，
并由 CI 步骤显式打印 `D6 Smoke Gate: DEGRADED`，与 PASS 行区分。

### 3.2 为什么排除 `tpch_gate_test`

缺 fixture 时它打印 `TPCH-NOT-RUN: reason=fixture_missing` 却仍报 ok。把它当硬目标等于
给门禁装一个永远绿的灯 —— 这正是 #3847 原本要消灭的那类"证明不了 PASS 却看起来 PASS"的问题。

### 3.3 为什么排除 `v312_13_load_data_sf1_test`

该目标在目标分支上为红（断言 `left: 346 / right: 630`，见 #3848）。它是全仓唯一真正生成
TPC-H fixture 并执行 LOAD DATA 的进程内目标，本应纳入冒烟，但**红目标不能当门禁目标**：
要么先修再回接，要么把整个分支的门禁拖红。裁定为**先修 #3848，再回接冒烟**，
中间态用 `#[ignore]` 掩盖被明确禁止（等于静默 PASS）。

### 3.4 退出码契约

| 门禁退出码 | 语义 | CI 步骤行为 |
|---|---|---|
| 0 | PASS | 打印 `D6 Smoke Gate: PASS` |
| 2 | DEGRADED | 打印 `D6 Smoke Gate: DEGRADED (conditional smoke target did not run; not a PASS)`，不阻塞 |
| 1 | FAIL | 打印 `D6 Smoke Gate: FAIL (exit N)` 并 `exit N`，保持阻塞 |

步骤用 `${PIPESTATUS[0]}` 取门禁自身退出码（而非 `tee` 的退出码），避免 `pipefail` 下
把 `tee` 成功误判为门禁成功。

## 4. 审核遗留项（不影响本 PR 放行）

1. `tpch_wire_smoke` 在 CI runner 上若无预编译二进制，将长期处于 DEGRADED 态。后续应给
   postcheck 增加 `cargo build -p sqlrustgo-mysql-server` 前置步骤（或复用制品），把 DEGRADED
   转为真 PASS。
2. #3848 修复后，应把 `v312_13_load_data_sf1_test` 作为条件目标回接冒烟集（届时二进制无关，
   可进硬集）。
3. 冒烟集硬目标若将来需要扩容，须同步修改 `test_inventory_gate_test.rs` 的规模断言，
   保持"小集合"这一防回归意图。
