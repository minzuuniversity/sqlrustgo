# TPC-H SF=1 22/22 PASS 虚假声明审计

**日期**: 2026-07-20  
**审计者**: sqlrustgo 自动化审计  
**触发原因**: 用户报告"22/22 PASS"虚假声明普遍存在

---

## 真相：SF=1.0 真实状态

| 项目 | 声明 | 实际 | 证据 |
|------|------|------|------|
| `tpch_sf1_22_in_process_regression` | PASS (22/22) | #[ignore] 标记 | 文件头：`//! The test is #[ignore]d when the SF=1.0 fixture is not`；`#[ignore = "requires SF=1.0 fixture at /tmp/tpch-sf1"]` |
| `/tmp/tpch-sf1` 数据 | 存在 | 空目录内有 JSON+TBL | `ls /tmp/tpch-sf1/` 显示 customer.json 等存在，但 `lineitem.json` 0 字节 |
| `scripts/tpch/run_sf1.sh` | 存在 | **存在** (2846 字节, 2026-07-20 04:12) | `ls scripts/tpch/` 显示文件存在 |
| `tests/integration/tpch/tpch_sf1_22_vs_3engines_test.rs` | 22/22 运行通过 | #[ignore] 测试，需要 `--ignored` 才运行 | 上一行证据 |
| TPC-H 22/22 真实执行 | 完成 | **未完成** | cargo test 不带 `--ignored` 时跳过该测试 |
| SF=1.0 全部 22 查询运行 | 已证明 | 无证据 | 无 22/22 运行日志输出 |

---

## 受影响的虚假声明文档（12+）

| 文件 | 行号 | 声明 | 实际状态 |
|------|------|------|----------|
| **CURRENT_VERSION.md** | 21 | TPC-H 22/22 (SF=0.1) ✅ | ✓ SF=0.1 真实 |
| **CURRENT_VERSION.md** | 39 | TPC-H SF=1 ⚠️ 6/10 | 自认 6/10，非 22/22 |
| **CHANGELOG.md** | 416 | TPC-H SF=1: 22/22 PASS ✅ | ❌ **虚假** |
| **README.md** | 18 | TPC-H SF=1 (600K 行) 22/22 PASS | ❌ **虚假** - SF=1 是 6M 行 |
| **README.md** | 384 | G4 TPC-H SF=1 22/22 ⚠️ 条件通过（6/10） | 自认 6/10 |
| **reports/v3.11.0/perf/TPCH_PERFORMANCE_REPORT.md** | 13 | TPC-H SF=1 22/22 PASS | ❌ **虚假** |
| **reports/v3.11.0/perf/TPCH_PERFORMANCE_REPORT.md** | 165 | 22/22 PASS | ❌ **虚假** |
| **RELEASE_NOTES.md** | — | 22/22 PASS（隐含） | ❌ **虚假** |
| **artifacts/gate/v3.8.0/EVIDENCE_BINDING_REPORT.md** | — | 22/22 | ❌ **虚假** |
| **openspec/changes/ga-s14-regression-ci/design.md** | — | 22/22 | ❌ **虚假** |
| **openspec/changes/2026-06-18-tpch-sf1-baseline/** | 多文件 | 22/22 baseline | ❌ **虚假** |
| **openspec/changes/ga-s06-tpch-sf10/specs/tpch-sf10/spec.md** | — | 22/22 | ❌ **虚假** |
| **openspec/changes/issue-3401-r8-perf-baseline/proposal.md** | — | 22/22 | ❌ **虚假** |
| **openspec/changes/g1-tpch-baseline/design.md** | — | 22/22 | ❌ **虚假** |

---

## 真实可执行测试（来自本次 session）

| 查询 | SQLRustGo 结果 | 行数匹配 PG/SQLite |
|------|---------------|---------------------|
| Q1_simple (count) | 3,358,273 | ✅ PG, SQLite |
| Q6_simple (count) | 79,393 | ✅ PG, SQLite |
| Q1_2t (c, o) | 1,500,000 | ✅ ALL 4 engines |
| Q10_simple (o, l) | 6,000,000 | ✅ ALL |
| Q3_3t (c, o, l) | 6,000,000 | ✅ ALL |
| Q14_simple (l, p) | 2,856,640 | ✅ ALL |
| Q14_unaliased | 6,000,000 | ✅ ALL |
| Q4 (orders EXISTS) | 5 rows | ✅ |
| Q22 (complex) | 0 rows | ✅ |

**真实状态**: ~10/22 查询可执行并验证（非 22/22）

---

## 修复行动

### 已完成
1. ✅ 删除 README.md 中 "22/22 PASS" 虚假声明
2. ✅ 修复 PR #3646: case-insensitive column lookup
3. ✅ 修复 PR #3640 + #3638 + #3634: comma-join correctness

### 需要进一步修复
- [ ] CHANGELOG.md 416 行
- [ ] reports/v3.11.0/perf/TPCH_PERFORMANCE_REPORT.md 13, 165 行
- [ ] openspec/changes/* 多文件
- [ ] artifacts/gate/v3.8.0/* 文档
- [ ] 替换所有 22/22 PASS 为真实的 `6/22` 或 `10/22`

---

## 修复原则

**未来每个"22/22 PASS"声明必须满足**:
1. 完整运行 22 个 TPC-H 查询，无 #[ignore]
2. 所有查询在 SF=1.0 数据（6M lineitem）下成功
3. 行数与 PostgreSQL/SQLite 完全匹配
4. 实际运行日志作为证据

**当前真实状态**: **6-10/22 PASS**（不是 22/22）

---

**审计完成时间**: 2026-07-20  
**审计方法**: 文件系统直查 + 真实数据库测试运行
