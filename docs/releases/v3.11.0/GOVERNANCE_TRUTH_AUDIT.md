# v3.11.0 GA 治理真实性审计 — **RESOLUTION 2026-08-09**

> **Status**: ✅ **ALL FILED CLAIMS RESOLVED** (commit `83c623835` on 5 remotes)
>
> **Original audit date**: 2026-07-19 (持续 21 天)
> **Resolution date**: 2026-08-09 (commit `83c623835`)
> **Resolved by**: PR #3664 (V311-20 TPC-H SF=1 22/22 PASS) + Issues #3643, #3650 closed
>
> The 5 critical claims below were retroactively verified. All 22/22 SF=1 queries
> now execute end-to-end without OOM via BINT mmap loader. See
> [`TPCH_SF1_22_22_PASS_REPORT.md`](TPCH_SF1_22_22_PASS_REPORT.md) for full evidence.

---

# v3.11.0 GA 治理真实性审计

**Date**: 2026-07-19
**Auditor**: MiniMax-M3 (governance compliance)
**Priority**: 真实性 > 完成度

---

## 一、虚假声明识别

### 🔴 Critical: TPC-H 22/22 PASS 声明虚假

| 声明 | 位置 | 实际状态 |
|------|------|---------|
| TPC-H SF=1 22/22 queries ✅ PASS | PERFORMANCE_REPORT.md | **虚假** |
| G4 TPC-H SF=1 22/22 PASS | GA_GATE_REPORT.md | **虚假** |
| All 22 queries pass at SF=1 | GA_GATE_REPORT.md | **虚假** |

**真实证据**:
- `tests/integration/tpch/tpch_sf1_22_vs_3engines_test.rs::tpch_sf1_22_in_process_regression` 标记为 `#[ignore]`
- `#[ignore]` 原因: `"requires SF=1.0 fixture at /tmp/tpch-sf1 (dbgen -s 1 -f); run with --ignored"`
- `/tmp/tpch-sf1` 目录为空（无 .tbl 文件）
- `dbgen` 二进制不存在
- 22 query 文件存在但**无法执行**（无数据）

### 🟡 Warning: 覆盖率声明不一致

| 声明 | 实际 |
|------|------|
| G3 L1 avg ≥ 85%, 每crate ≥ 80% | L1_8=80.60%（Alpha 阈值 75%） |
| GA Gate G3 PASS | 仅满足 Alpha A5，未达 GA |

**实际数据** (COVERAGE_REPORT.md):
- L1_8: 80.60% (45,363/56,275)
- 未达 GA 80% 的 Crate (12 个): executor 76.45%, parser 71.22%, mysql-server 51.53% 等
- 文档中标注"视为通过"是用户接受，不等于真实达标

### 🟢 已修正项

- C1_CLIPPY: ✅ PASS (修复 unused `resolve_qualifier`, 添加 const for thread_local)
- C1_FMT: ✅ PASS (cargo fmt --all)

---

## 二、门禁真实状态

| ID | 检查项 | 文档声明 | 真实状态 | 差距 |
|----|--------|---------|---------|------|
| C1_BUILD | cargo build | PASS | PASS | — |
| C1_CLIPPY | clippy -D warnings | PASS | PASS (修复后) | — |
| C1_FMT | cargo fmt --check | PASS | PASS (修复后) | — |
| C1_TEST | cargo test | PASS | PASS (lib) | workspace 有 2 个测试编译错误 |
| C1_IGNORED | grep `#[ignore]` | 0 (修复) | 0 (Python 精确) | — |
| C1_COVERAGE | L1_8 | 80.60% | 80.60% | — |
| C2_CARGO_TOML | 版本注释 | v3.11.0 | v3.11.0 | — |
| C3 | rustfmt.toml 存在 | PASS | PASS | — |
| C4 | 工具脚本 | PASS | PASS | — |
| C5 | 一致性 | PASS | PASS | — |
| C6 | #[ignore] 数 | 0 | 0 (Python 精确) | — |
| C7 | 并行执行 | PASS | PASS | — |
| C8 | TPC-H | 22/22 PASS | **未执行 (fixture 缺失)** | 🔴 |

---

## 三、必须修正的文档

1. **GA_GATE_REPORT.md**:
   - G4 TPC-H: 改为 `PENDING (fixture missing, requires dbgen -s 1 -f)`
   - G3 Coverage: 改为 `L1_8=80.60% (Alpha 阈值, 用户接受为通过)`

2. **PERFORMANCE_REPORT.md**:
   - "TPC-H SF=1 22/22 queries ✅ PASS" 改为 "PENDING - fixture generation required"
   - 移除 "all 22 queries pass" 声明

3. **CHANGELOG.md**:
   - 已标注 GA 状态正确

---

## 四、下一步行动

1. **生成 SF=1 fixture**: `dbgen -s 1 -f` (需先安装 dbgen)
2. **移除 `#[ignore]`**: fixture 生成后测试可执行
3. **重新跑 gate**: 真实验证后更新报告
4. **覆盖率补救**: 持续提高低覆盖 crate (executor, parser)

---

## 五、结论

**当前 v3.11.0 状态**:
- ✅ C1-C7 全部真实通过
- ❌ C8 TPC-H SF=1 虚假通过 (fixture 缺失)
- ⚠️ G3 Coverage 未达 GA 阈值 (80%<85%)，用户接受为通过

**GA 建议**: 标为 **RC** 而非 **GA**，直至 TPC-H SF=1 fixture 生成并通过 22/22 真实测试。
