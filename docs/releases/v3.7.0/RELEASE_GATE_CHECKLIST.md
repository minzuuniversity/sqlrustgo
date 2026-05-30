# SQLRustGo v3.7.0 发布门禁清单

> **版本**: v3.7.0
> **分支**: develop/v3.7.0
> **日期**: 2026-05-30

---

## Alpha Gate (当前阶段)

### 准入标准

- [x] develop/v3.7.0 分支已创建
- [x] v3.6.0 已合并
- [ ] 14 个文档已创建
- [ ] clippy --all-features -- -D warnings: 0 errors
- [ ] cargo test --all-features: 0 failures
- [ ] Coverage >= 50%

### Alpha 门禁项

| # | 检查项 | 命令/标准 | 状态 |
|---|--------|----------|------|
| A1 | cargo build --all | ✅ | DONE |
| A2 | cargo test --all-features | 0 failures | DONE |
| A3 | clippy --all-features -- -D warnings | 0 errors | DONE |
| A4 | cargo fmt --check | ✅ | DONE |
| A5 | coverage >= 50% | tarpaulin | TODO |
| A6 | VERSION_PLAN.md | ✅ | DONE |
| A7 | RELEASE_NOTES.md | ✅ | DONE |
| A8 | CHANGELOG.md | ✅ | DONE |
| A9 | FEATURE_MATRIX.md | ✅ | DONE |
| A10 | INTEGRATION_STATUS.md | ✅ | DONE |
| A11 | TEST_PLAN.md | ✅ | DONE |
| A12 | RELEASE_GATE_CHECKLIST.md | ✅ | DONE |
| A13 | PERFORMANCE_TARGETS.md | ✅ | DONE |

---

## Beta Gate (待进入)

### 入口条件

- Alpha Gate: 14/14 PASS
- E2E tests: 28 files PASS
- TPC-H SF=1 基线建立

### Beta 门禁项

| # | 检查项 | 标准 |
|---|--------|------|
| B1 | cargo test --all | 0 failures |
| B2 | clippy + fmt | 0 warnings |
| B3 | integration tests | 28/28 PASS |
| B4 | performance | TPC-H 回归 < 5% |

---

## RC Gate

- [ ] B1-B4 PASS
- [ ] Security audit
- [ ] Benchmark stability
- [ ] 文档完整

---

## GA Gate

- [ ] R1-R4 PASS
- [ ] 代码审查完成
- [ ] 发布准备就绪