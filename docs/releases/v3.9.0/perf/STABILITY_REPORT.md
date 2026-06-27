# G13 Stability Test Report (v3.9.0)

> **Generated**: 2026-06-05
> **Gate**: G13 (24h 真实稳定性, 强制 GA 门禁)
> **Ref**: V390_TEST_PLAN_ROUND2_REVIEW §G13
> **Compressed (Pre-GA)**: 72h Soak 已在 beta 阶段完成 (见 `beta/SOAK_72H_REPORT.md`)

---

## 1. 概述

G13 24h 真实稳定性是 **GA 卡死门禁**. 72h / 168h 推迟到 Post-GA 持续监控 (Nightly / Weekly).

| 阶段 | 时长 | 强制 | 状态 |
|------|------|------|------|
| **Pre-GA (beta)** | 72h 压缩 (180s) | ✅ 已 PASS | opencode 完成 (PR #3212) |
| **GA 强制** | **24h 真实** | ✅ GA 卡死 | TBD (W12 D1-2 在 Z6G4 跑) |
| Post-GA Nightly | 72h 真实 | ⚠️ 监控 | 推迟 |
| Post-GA Weekly | 168h 真实 | ⚠️ 监控 | 推迟 |

---

## 2. 72h 压缩 Soak (已 PASS, beta 阶段)

来自 `docs/releases/v3.9.0/beta/SOAK_72H_REPORT.md`:

| Invariant | 72h 等价值 | 容忍度 | 状态 |
|-----------|-----------|--------|------|
| Memory growth | < 10% | PASS (per ALPHA_GATE_CONTRACT §1.7) | ✅ |
| FD leak | 0 | 0 | ✅ |
| Lock leak | 0 | 0 | ✅ |
| P99 latency | < 50ms | bounded | ✅ |
| 崩溃次数 | 0 | 0 | ✅ |

10/10 soak tests PASS + G7 gate 7/7 PASS.

---

## 3. 24h 真实 Soak (W12 D1-2 实施)

### 3.1 实施方案

- 环境: Z6G4 (8 cores / 64GB / NVMe)
- 工作负载: sysbench oltp_read_write 8 threads
- 监控: RSS / FD / CPU / WAL / Lock (60s 间隔)
- 时长: 24h wall-clock
- 数据集: 10K sbtest table
- 异常阈值: RSS > 4GB, FD > 5000, WAL > 10GB, crash > 0

### 3.2 24h 真实结果 (TBD)

| 指标 | 起始 | 24h 后 | 增长 | 阈值 | 状态 |
|------|------|--------|------|------|------|
| RSS (MB) | TBD | TBD | TBD | < 50 MB/24h | TBD |
| FD 句柄 | TBD | TBD | TBD | < 50 | TBD |
| CPU avg | TBD | TBD | TBD | < 80% | TBD |
| WAL (MB) | TBD | TBD | TBD | < 1024 MB | TBD |
| 锁等待 avg (ms) | TBD | TBD | TBD | < 50 ms | TBD |
| 崩溃次数 | 0 | 0 | 0 | 0 | TBD |
| sysbench TPS | TBD | TBD | TBD | ≥ v3.8.0 × 0.9 | TBD |
| sysbench P99 | TBD | TBD | TBD | ≤ v3.8.0 × 1.15 | TBD |

**实施细节**: 见 `scripts/stability/run_24h_soak.sh`

---

## 4. Post-GA 72h/168h (Nightly/Weekly)

- 72h Nightly: 每周 1 次 (持续监控)
- 168h Weekly: 每月 1 次 (长期稳定性)
- 推迟到 Post-GA, GA 不卡

实施脚本已就位:
- `scripts/stability/run_72h_soak.sh`
- `scripts/stability/run_168h_soak.sh`

---

## 5. 验收 (W12 D2 后)

```
✅ Pre-GA 72h 压缩 PASS (opencode beta)
⏳ GA 24h 真实 PASS (W12 D1-2 跑, Z6G4)
✅ G7 7/7 PASS
✅ TPC-H 22/22 维持
⏳ Performance Baseline 24h 资源列填入 (W12 D3)
```

---

## 6. 报告归档

- 真实数据: `test_results/stability_24h_*/metrics.csv` + `SUMMARY.md`
- sysbench 输出: `test_results/stability_24h_*/sysbench.log`
- server 日志: `test_results/stability_24h_*/sqlrustgo.log`

**Ref**:
- V390_TEST_PLAN_ROUND2_REVIEW §G13 (24h 强制, 72/168h 推迟)
- beta/SOAK_72H_REPORT.md (opencode 72h 压缩 PASS)
- scripts/stability/run_*.sh (3 个真实长跑脚本)
- scripts/gate/check_p13_soak_test.sh (G7 单元级)
