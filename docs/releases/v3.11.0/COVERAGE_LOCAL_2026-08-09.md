# v3.11.0 覆盖率实测对比 — `--lib` vs `--tests` (2026-08-09 本地)

> **生成时间**: 2026-08-09
> **环境**: 本地工作站（cargo 1.x, llvm-cov 0.8.7, rustc stable）
> **测量命令**: `cargo llvm-cov --release -p <crate> --all-features [--lib|--tests] --no-fail-fast`
> **分支**: `develop/v3.11.0` @ `a13557b146`

## 4 个 L1_8 失败 crate 的 `--lib` vs `--tests` 实测对比

| Crate | `--lib` (GA 门标准) | `--tests` (ADR-001 G-04 SSOT) | Δ | 是否过 80% 阈值 |
|-------|---------------------|-------------------------------|-----|------------------|
| sqlrustgo-executor | 73.71% (本地) / 76.41% (COVERAGE_FULL) | 未测（编译期有 3 个 binary 失败，待修） | — | ❌ 两侧均失败 |
| sqlrustgo-admin | **65.08%** | **82.99%** | **+17.91pp** | ✅ `--tests` 翻 ✅ |
| sqlrustgo-mysql-server | 40.45% / 42.91% | **50.87%** | **+10.42pp** | ❌ 两侧均失败 |
| sqlrustgo-mysql-client | 31.56% / 31.42% | **43.79%** | **+12.23pp** | ❌ 两侧均失败 |

> **注**: executor 本地 `--lib` 实测 73.71% 略低于 COVERAGE_FULL 2026-08-09 的 76.41%，原因可能是
> 本地用了 `RUSTFLAGS=--cfg=skip_long_tests` 跳过了一些长跑测试；但量级与官方一致。

## 结论

1. **`--tests` 是有效的杠杆**：admin 从 65.08 → 82.99pp（+17.91pp）直接翻 80% 阈值。
2. **`--tests` 不够**：mysql-server + mysql-client 仍距离 80% 阈值 -29.13pp / -36.21pp；需要补 inline 测试或修复测试 bug。
3. **`--lib` 与 `--tests` 的差距**：12–18pp，证实 `coverage-baseline/README.md` 标注的 ADR-001 G-04 是有意义的。

## 顺手发现的测试 bug（应单独追踪）

| 测试 | 失败原因 | 状态 |
|------|----------|------|
| `mysql-server::test_binary_storage_tpch_sf1_load` | 需要 1.1GB SF=1 fixture，`/tmp/tpch-sf1` 仅 symlink 到 `/home/openclaw/tpch_baseline/sf1`，fixture 检查失败 | 环境问题，非产品 bug |
| `mysql-server::tests::test_e2e_select_multiple_columns_rows` | parser 拒绝 `SELECT 1 AS id, 'hello' AS name, 3.14 AS value UNION ALL SELECT 2, 'world', 2.71`：`Expected FROM or column name` | **产品 bug**: UNION ALL parse error |
| `mysql-server::tests::test_execution_engine_state_persistence` | parser 拒绝 SET 子句：`Expected column name in SET` | **产品 bug**: SET syntax parse error |
| `executor::tests::full_outer_join_test` (3 tests) | 编译期失败（运行前 panic） | 需诊断 |
| `executor::tests::hash_join_left_null_test` (1 test) | 编译期失败 | 需诊断 |
| `executor::tests::test_stored_proc` (1 test) | 编译期失败 | 需诊断 |

## 建议

1. **采纳 ADR-001 G-04**：把 GA 门从 `--lib` 切到 `--tests`。**admin 一项立即通过**。
2. **后续工作**：补 mysql-server + mysql-client 的 inline 测试（per `G3_COVERAGE_REMEDIATION_PLAN.md`）。
3. **单独追踪**：上述 3 个产品 parser bug（UNION ALL / SET / 3 个 executor test 编译失败），开 issue 或独立 PR。

## 验证

```bash
# admin 已验证
cargo llvm-cov --release -p sqlrustgo-admin --all-features --tests --no-fail-fast
# → TOTAL 82.99% line ✅

# mysql-server 已验证（skip 3 个 binary 失败测试）
cargo llvm-cov --release -p sqlrustgo-mysql-server --all-features --no-fail-fast \
  --lib --test mysql_server_helper_tests --test mysql_server_unit_tests \
  --test server_test --test prepared_stmt_params_test --test e2e_wire_protocol \
  --test mysql_server_tests -- \
  --skip test_binary_storage_tpch_sf1_load \
  --skip test_e2e_select_multiple_columns_rows \
  --skip test_execution_engine_state_persistence
# → TOTAL 50.87% line ❌

# mysql-client 已验证
cargo llvm-cov --release -p sqlrustgo-mysql-client --all-features --tests --no-fail-fast
# → TOTAL 43.79% line ❌
```

## 配套 ADR-008 异常文件

## 后续更新：合并 252 Gitea 最新提交后 (2026-08-09)

拉取并合并 252 上 2 个未在 develop/v3.11.0 上的分支：
- `fix/f23-clustered-dml-routing`：1 commit (2c67074ba3 fmt rustfmt to expr/mod.rs)
- `coverage-tests-2026-08-09b`：1 commit (f2323a497d test executor+server coverage for mutation/predicate/update compiler + security_integration)

合并后 HEAD = `966e603296`，G3 `--lib` 重测：

| Crate | 合并前 | 合并后 | Δ | 状态 |
|-------|--------|--------|---|------|
| executor | 72.78% | **74.23%** | +1.45pp | ❌ (差 -5.77pp) |
| admin | 81.78% | **81.78%** | 0 | ✅ |
| mysql-server | ~40% | (test_binary_storage_tpch_sf1_load 仍失败) | — | ❌ |
| mysql-client | 31.56% | **31.56%** | 0 | ❌ |

3 个 product parser bug 经合并后**仍然存在**：
1. `mysql-server::tests::test_binary_storage_tpch_sf1_load`（SF=1 fixture 环境问题）
2. `mysql-server::e2e_wire_protocol::test_e2e_select_multiple_columns_rows`（UNION ALL parse error）
3. `mysql-server::tests::test_execution_engine_state_persistence`（SET clause parse error）


## 后续更新 (2026-08-09 晚): G3 进度

| Crate | --lib (start) | --lib (after) | --tests (after) | 阈值 | 状态 |
|-------|---------------|----------------|-----------------|------|------|
| executor | 72.78% | 74.90% | **78.12%** | 80% | ❌ (--lib ❌, --tests 差 1.88pp) |
| admin | 81.78% | 81.78% | 82.99% | 80% | ✅ |
| mysql-server | 40.45% | 47.39% | 50.87% | 80% | ❌ |
| **mysql-client** | 31.56% | **84.56%** | 更高 (估算 >85%) | 80% | ✅ |

G3 现状：**2/4 通过** (admin + mysql-client)。mysql-server 差距大 (47.39% → 需要 +32pp 到 80%)。

**关键路径**：执行 `cargo llvm-cov --release -p <crate> --all-features --tests --no-fail-fast`
而不是 `--lib`，按 ADR-001 G-04 的 SSOT。`--tests` 自动包含 `crates/<crate>/tests/*.rs` 中
的 27 (executor) / 8 (admin) / 6 (mysql-server) / 1 (mysql-client) 集成测试。
`docs/governance/adr/ADR-008-exception-v311-tpch-sf1.md` 已创建并提交。
覆盖 v3.11.0 G4 门至 2026-09-01 ADR-008 §Policy 2 例外（详见 `G4_WIRE_TEST_CLOSE_OUT_PLAN.md`）。