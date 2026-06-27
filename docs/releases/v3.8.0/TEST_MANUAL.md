# SQLRustGo v3.8.0 测试手册

> **版本**: v3.8.0
> **发布日期**: 2026-06-04

---

## 测试层级

### L1: 单元测试

```bash
cargo test --lib --all-features
```

### L2: 集成测试

```bash
cargo test --test integration_tests --all-features
```

### L3: 门禁测试

```bash
# Alpha Gate
bash scripts/gate/check_alpha_gate.sh

# Beta Gate
bash scripts/gate/check_beta_gate.sh

# RC/GA Gate
bash scripts/gate/check_rc_ga_gate.sh
```

---

## RECOVERY 测试

```bash
cargo test --test wal_tx_contract_test --all-features
# 期望: 22 passed; 0 failed; 2 ignored
```

---

## TPC-H 测试

```bash
cargo build --release -p sqlrustgo-bench-cli
./target/release/sqlrustgo-bench-cli tpch-bench --queries all
# 期望: 13/22 PASS
```

---

## 覆盖率检查

```bash
bash scripts/gate/check_coverage.sh
# 期望 L1: 8 crates average >= 75%
```
