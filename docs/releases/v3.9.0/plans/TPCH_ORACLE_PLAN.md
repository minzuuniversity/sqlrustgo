# TPC-H Oracle System Plan (v3.9.0)

> **Generated**: 2026-06-07
> **Ref**: 用户 2026-06-07 "建立证据"模式 → 专项 A TPC-H Correctness
> **Target**: 把 TPC-H 正确性从 40% (5/22 mismatch) → 100% (22/22 oracle match)

---

## 1. 目标 (Why)

**当前**:
- 4-way G17 5/22 mismatch (Q20/Q21 architecture complete but row count wrong + Q6/Q19 PG-only)
- 4 engines 比对 (sqlrustgo vs SQLite vs MariaDB vs PostgreSQL)
- "Match" 仅当 4 个 engine 行数完全相同

**目标**:
- 引入 **Oracle (权威答案)** 取代 4-way mutual comparison
- Oracle 引擎: **DuckDB** (首选, 静态链接, 单 binary, 嵌入式)
- 4-way 演变为 5-way (sqlrustgo vs SQLite vs MariaDB vs PostgreSQL vs DuckDB)
- "Match" 改为 vs **DuckDB oracle** (权威), 4-way 仍 useful as cross-check

---

## 2. 设计 (How)

### 2.1 DuckDB 集成 (A1)

```toml
# Cargo.toml (新增)
[dependencies]
duckdb = { version = "1.1", features = ["bundled"] }
```

Bundled 静态链接, 无外部依赖。

### 2.2 Oracle 测试文件

```
testdata/tpch/oracle/
├── setup.sql                # 22 query + DDL
├── q01.sql                  # SELECT l_returnflag, l_linestatus, ...
├── q02.sql                  # SELECT s_acctbal, s_name, ...
...
├── q22.sql                  # SELECT cntrycode, ...
└── expected/
    ├── q01.txt              # canonical SF=1 result rows
    ├── q02.txt
    ...
    └── q22.txt
```

`expected/qNN.txt` 由 DuckDB 在 reference 数据集上生成一次 (signed off 后 commit).

### 2.3 Oracle 加载器 (A2: Query Diff Framework)

```rust
// crates/oracle/src/lib.rs
pub fn load_expected_results(data_dir: &Path) -> BTreeMap<u8, Vec<Vec<Value>>> {
    // 读 testdata/tpch/oracle/expected/q01.txt
}

pub fn run_duckdb_oracle(data_dir: &Path, query: &str) -> Vec<Vec<Value>> {
    // 动态创建 DuckDB 临时 instance, load .tbl, run query, return rows
}
```

### 2.4 Cargo test 集成 (A2)

```rust
// tests/tpch_diff_test.rs (新)
#[test]
fn test_tpch_22_vs_duckdb_oracle() {
    for q in 1..=22 {
        let expected = load_expected_results(...);
        let actual = run_duckdb_oracle(...);
        if !rows_match(&expected, &actual) {
            panic!("Q{q} mismatch");
        }
    }
}
```

### 2.5 集成到 PR Gate (A4)

```bash
# scripts/gate/check_tpch_correctness.sh
cargo test --release --test tpch_diff_test -- --nocapture
# 必须 22/22 PASS
```

集成到 `gate.sh` (主 gate), 任何 PR 触发。

---

## 3. 步骤 (When)

| Day | 任务 | 输出 |
|----|----|----|
| D1 | DuckDB 集成 (Cargo.toml + lib.rs) | `cargo build -p oracle` OK |
| D2 | 写 oracle 加载器 | `load_expected_results` + `run_duckdb_oracle` |
| D3 | 生成 22 query expected/QNN.txt (signed off) | testdata 完整 |
| D4 | 写 `tests/tpch_diff_test.rs` | 5-way test pass with DuckDB oracle |
| D5 | Gate integration | `scripts/gate/check_tpch_correctness.sh` |

---

## 4. 风险 (Risk)

| 风险 | 缓解 |
|----|----|
| DuckDB 静态链接 binary 大 (10-20 MB) | 可选 features, CI build only |
| DuckDB 与 SQLite/MariaDB/PG 的 SQL dialect 差异 (EXTRACT, NULL handling) | 用 canonical TPC-H spec, 不看 SQLite/MariaDB/PG |
| SF=1 数据集准备 | 已有 `tpch_data_gen` (lineitem 16 列) |
| 22 query expected QNN.txt 标定 | 用 DuckDB 在 1 套 reference dataset 上跑, commit once |

---

## 5. 验收 (Acceptance)

- [ ] `cargo test --test tpch_diff_test` 22/22 PASS
- [ ] Gate script `check_tpch_correctness.sh` exit 0
- [ ] 252 Gitea 恢复后 PR + merge
- [ ] Issue #3248 (Q20/Q21) close after Q20/Q21 行数 = DuckDB oracle
- [ ] 22/22 Q oracle 比对 PASS = v3.9.0 TPC-H 正确性 100% (P0-1 ✅)

---

**Status**: 📋 Plan ready, 等待 252 Gitea 恢复后开 PR 启动.
**Owner**: TPC-H Tiger Team (Team-A)
