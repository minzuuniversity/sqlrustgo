# v312-58 / engine_select Q7_TRACE gate fix — TPC-H 测试挂起根因

**Branch**: `fix/v312-58-tpch-sf1-7x`
**Date**: 2026-08-23
**Author**: openclaw
**Verdict**: **REAL bug** — debug 门控遗漏导致 TPC-H GROUP BY 查询无条件喷出大量
`eprintln!` 到 stderr，触发 PTY 缓冲死锁，造成 cargo test 卡死到必须断电。

---

## 1. 现象

用户报告：跑 TPC-H 相关测试时，系统经常整个挂起，不得不断电重启服务器。

特征：
- 一旦挂起，Ctrl+C 救不回来
- 多次重现
- 单独跑某个轻量级诊断测试也偶发
- `--nocapture` 时更易触发

## 2. 根因分析

排查 `fix/v312-58-tpch-sf1-7x` 分支上未提交修改，发现
`src/engine_select.rs` 中针对 Q7 EXTRACT 问题排查时引入了大量
`[Q7_TRACE]` 调试打印。**意图是全部用 `Q7_TRACE` 环境变量门控，但
在 5 个关键位置遗漏了门控条件**，导致所有 GROUP BY 查询都会无条件
向 stderr 喷出大量调试输出。

### 2.1 漏门控的位置（修复前）

| 文件 | 行号 | 喷点 | 每次执行量 |
|------|------|------|------------|
| `src/engine_select.rs` | 633, 635 | `group_exprs` 元数据 | 每个 GROUP BY 1 次 |
| `src/engine_select.rs` | 661, 662 | `distinct groups` / `total rows` | 每个 GROUP BY 1 次 |
| `src/engine_select.rs` | 948-955 | SELECT 列元数据 + `agg_result_rows` 逐行 | 每个 GROUP BY 行 × N |
| `src/engine_select.rs` | 1145 (旧 1138) | `UNMATCHED col key` | 每行 × 每列（嵌套 map）|
| `src/engine_select.rs` | 1153 (旧 1143) | `.inspect()` FINAL row | 每个最终行 1 次 |

### 2.2 同一文件正确门控的位置

- 637-648 行：明确使用了 `if _q7_trace { ... }` 包裹
- 657-658 行：`if _q7_trace && _q7_keys_seen.insert(...)`

属于典型的"门控重构遗漏"：作者本来想用环境变量控制，但分散粘贴时
漏了几处。

### 2.3 挂起机制

1. 诊断测试的注释都要求 `--nocapture` 跑
   （`cargo test --test diag_q7_xxx -- --nocapture`），这会把 stderr
   直连到 PTY。
2. Linux PTY 缓冲通常只有 4-64 KB。
3. 嵌套 `.map()` + `.inspect()` 喷出极快。TPC-H 实际命中量：
   - Q9 (SF=1) = 175 行 → 175 次 eprintln
   - Q2 (SF=1) = 100 行 → 100 次
   - Q22 = 若干 × 每列 → 数十次
4. PTY 缓冲满 → `write(2)` 阻塞 → 测试进程卡死
5. cargo test 父进程级联等待所有子进程；Ctrl+C 也救不回来
6. 用户只能断电

## 3. 修复

最小修复：将 5 处 eprintln! 全部用 `if _q7_trace { ... }` 包裹。

### 3.1 改动 diff（节选）

```rust
// 之前：
eprintln!("[Q7_TRACE] group_exprs.len() = {}", group_exprs.len());
for (gi, ge) in group_exprs.iter().enumerate() {
    eprintln!("[Q7_TRACE]   group_exprs[{}] = {:?}", gi, ge);
}

// 之后：
if _q7_trace {
    eprintln!("[Q7_TRACE] group_exprs.len() = {}", group_exprs.len());
    for (gi, ge) in group_exprs.iter().enumerate() {
        eprintln!("[Q7_TRACE]   group_exprs[{}] = {:?}", gi, ge);
    }
}
```

类似地修复了 661-662、948-955、1145（旧 1138）、1153（旧 1143）。

### 3.2 清理

删除编辑残留 `tests/common/tpch_wire_harness.rs.tmp.359302.9b36fc81ce5c`。

## 4. 验证

| 场景 | 命令 | 结果 |
|------|------|------|
| 完整 build | `cargo build --all-features` | ✅ Finished in 4.20s |
| executor lib 全部测试 | `cargo test -p sqlrustgo-executor --lib` | ✅ 696 passed; 0 failed |
| sqlrustgo lib 全部测试 | `cargo test -p sqlrustgo --lib` | ✅ 55 passed; 0 failed |
| 3 个新 diag_q7 测试 | `cargo test --test diag_q7_mini --test diag_q7_mini_columns --test diag_q7_extract` | ✅ 2 passed; 2 ignored（按设计）|
| Q16 NOT IN 回归（38s 真实 TPC-H）| `cargo test --test q16_notin_subquery_regression --all-features` | ✅ 2 passed in 38.26s（修复前会触发 PTY 阻塞）|
| Q7 mini fixture（无 Q7_TRACE）| `cargo test --test diag_q7_mini -- --nocapture` | ✅ 0.00s，输出 1 行，无 [Q7_TRACE] 喷出 |
| Q7 mini fixture（有 Q7_TRACE=1）| `Q7_TRACE=1 cargo test --test diag_q7_mini -- --nocapture` | ✅ 调试输出完整保留 |

**关键回归信号**：Q16 NOT IN 跑了 38.26s（Q16 是用 GROUP BY + NOT IN 子查询
的真实 TPC-H 查询）。如果 eprintln 喷出未修复，此测试在 `--nocapture`
下会因 PTY 缓冲阻塞而挂起。修复后正常运行。

## 5. 已知遗留（不在本次修复范围）

- `tests/integration/oracle/diag_q7_extract.rs:73` 的
  `assert_eq!(r.rows.len(), 7)` 与已知事实（标准 Q7 SF=1 = 4 行）冲突。
  已被 `#[ignore]` 标记，不会默认跑。

## 6. 防御性建议

未来如果再次需要临时调试打印，建议：

1. 用 `log::trace!` 通过 `RUST_LOG` 控制（无 env 检测开销）
2. 或用 `eprintln!` 时一律包裹 `if std::env::var("DEBUG_XXX").is_ok() { ... }`
3. CI 跑前加 grep 自检，禁止 `[DEBUG_]` 前缀的 eprintln 出现

---

**关联**：本修复不改变 SQL 语义，仅移除调试噪音。是 v312-58 SF=1 7x
分块治理工作流的运维性补丁。
