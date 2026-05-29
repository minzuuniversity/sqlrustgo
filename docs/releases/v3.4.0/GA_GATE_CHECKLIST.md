# v3.4.0 GA Gate Checklist（完整版）

> **版本**: v2.0
> **创建日期**: 2026-05-25
> **最后更新**: 2026-05-26
> **维护人**: hermes-agent
> **分支**: `origin/develop/v3.4.0`
> **HEAD**: `024d12e9`

---

## 一、检查清单使用说明

本清单是 v3.4.0 GA 门禁检查的规范来源（SSOT）。每次执行 GA Gate 前必须通读本清单。

**通过标准**: 45 PASS / 3 SKIP / 0 FAIL（无 FAIL 项）

**Truthfulness 要求**: 所有检查项必须实际执行命令，禁止跳过步骤写结论。

---

## 二、Gate 脚本映射

| 检查项 | 命令 | Makefile 目标 |
|--------|------|--------------|
| G1 Build | `cargo build --release --workspace` | `make build` |
| G2 Test | `cargo test --all-features --lib` | `make unit` |
| G3 Clippy | `cargo clippy --all-features -- -D warnings` | `make clippy` |
| G4 Format | `cargo fmt --all -- --check` | `make fmt` |
| G5 Coverage | `cargo llvm-cov test -p sqlrustgo-* --lib` | `make coverage` |
| G6 Security | `cargo audit` | `make audit` |
| G7~G9 GMP Build | `cargo build -p sqlrustgo-gmp-* --release` | `make build` |
| G10 TPC-H SF=1 | `bash scripts/gate/check_tpch.sh --sf1` | `make benchmark` |
| G11 Proofs | `find docs/proof -name "*.json" -type f \| wc -l` | - |
| G12 Docs | `ls docs/releases/v3.4.0/oo/` | - |
| G-TI9 QPS | `bash scripts/gate/check_perf_baseline.sh` | `make perf` |
| G-CR4 72h | `cargo test --test long_run_stability_72h_test -- --ignored` | `make stability` |
| G-CR5 Smoke | `cargo test --test engine_test` | `make smoke` |
| G-FZ1 Fuzz | `cargo run -p sqlrustgo-fuzz --bin row_format_fuzz -- 10000` | `make fuzz` |
| G-FZ2 Fuzz | `cargo run -p sqlrustgo-fuzz --bin row_format_fuzz -- 1000` | `make fuzz-quick` |
| G-CH1~3 Chaos | `bash scripts/chaos/test_*.sh` | `make chaos` |
| G-SF10 TPC-H | `sqlrustgo-bench-cli tpch-bench --scale 10` | `make tpch-sf10` |

---

## 三、Gate v2.0 新增项（vs v1.0）

| 新增项 | 类型 | 测试内容 |
|--------|------|---------|
| G-TI9 | 性能回归 | QPS baseline vs current |
| G-CR4 | 72h稳定性 | abbreviated 60s check |
| G-CR5 | 冒烟测试 | engine_test sanity |
| G-FZ1 | Fuzz | 10k rounds row format fuzz |
| G-FZ2 | Fuzz | 1k rounds quick fuzz |
| G-FZ3 | Fuzz | SQL compatibility (sqlite_diff) |
| G-CH1 | Chaos | OOM simulation |
| G-CH2 | Chaos | I/O error injection |
| G-CH3 | Chaos | Crash simulation |
| G-SF10 | TPC-H | SF=10 stress test |

---

## 四、门禁执行规范

1. **顺序执行**: 按 G1 → G2 → ... → G-SF10 顺序执行
2. **实际命令**: 每个检查项必须执行实际命令并捕获输出
3. **失败处理**: 任何 FAIL 项必须立即停止并修复，不允许带病发布
4. **SKIP 处理**: SKIP 项需记录原因并在 48h 内补充验证

---

## 五、门禁结果阈值

| 指标 | 要求 | 当前值 |
|------|------|--------|
| PASS 率 | >= 45 | 45 |
| FAIL 容忍 | 0 | 0 |
| SKIP 上限 | <= 3 | 3 |
| Truthfulness | >= 95% | 100% |
| 覆盖率 | >= 75% (L1) | 83.14% |

---

## 六、问题升级路径

| 严重级别 | 定义 | 处理时限 |
|---------|------|--------|
| P0 | Gate FAIL / 安全漏洞 | 立即停止发布 |
| P1 | SKIP 项 > 3 | 24h 内解决 |
| P2 | Truthfulness < 95% | 48h 内解决 |
| P3 | 覆盖率 < 75% | 下个版本迭代 |

---

*最后更新: 2026-05-26 | v2.0*
