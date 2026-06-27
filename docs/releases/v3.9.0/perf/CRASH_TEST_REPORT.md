# G14 Real Crash Test Report (v3.9.0)

> **Generated**: 2026-06-05
> **Gate**: G14 (真实进程级崩溃注入, GA 强制)
> **Ref**: V390_TEST_PLAN_ROUND2_REVIEW §G14
> **Mock (Pre-GA)**: G8 Crash Matrix 100+ scenarios PASS (Hermes #3174)

---

## 1. 概述

G14 真实崩溃测试是 **GA 卡死门禁** (8 类真实进程级崩溃注入). G8 (单元 mock) 仅 CI 用, G14 必须真实运行才能 GA.

| 阶段 | 类型 | 强制 | 状态 |
|------|------|------|------|
| **Pre-GA (CI)** | G8 单元 mock (100+ scenarios) | ✅ 已 PASS | Hermes #3174 |
| **GA 强制** | **G14 真实 8 类** | ✅ GA 卡死 | TBD (W12 D3-4 在 Z6G4 跑) |

---

## 2. 8 类真实崩溃场景

| # | 场景 | 真实命令 | 验证 |
|---|------|----------|------|
| 1 | **SIGKILL mid-INSERT** | `kill -9 $PID` 期间 sysbench | TPC-H 22/22 维持 |
| 2 | **SIGKILL mid-COMMIT** | 同上, COMMIT flush 中 | 同上 |
| 3 | **SIGKILL mid-ROLLBACK** | 同上, ROLLBACK TO SAVEPOINT | 同上 |
| 4 | **Power loss (rm WAL)** | `rm WAL/* ; restart` | 同上 |
| 5 | **Disk full (fallocate 100M)** | `fallocate -l 100M WAL/x` | 同上 |
| 6 | **OOM (cgroup)** | `cgroup memory.limit=100M` | 同上 |
| 7 | **WAL corruption (字节翻转)** | `dd conv=notrunc bs=1 seek=N < /dev/urandom` | 同上 |
| 8 | **Process hang (kill -STOP)** | `kill -STOP $PID` (sleep 10) | 同上 |

实施细节见 `scripts/crash/run_*.sh`.

---

## 3. 真实运行结果 (TBD, W12 D3-4 跑)

| Case | 启动 | 崩溃注入 | 重启 | TPC-H | 状态 |
|------|------|----------|------|-------|------|
| 1. SIGKILL mid-INSERT | TBD | TBD | TBD | TBD | TBD |
| 2. SIGKILL mid-COMMIT | TBD | TBD | TBD | TBD | TBD |
| 3. SIGKILL mid-ROLLBACK | TBD | TBD | TBD | TBD | TBD |
| 4. Power loss | TBD | TBD | TBD | TBD | TBD |
| 5. Disk full | TBD | TBD | TBD | TBD | TBD |
| 6. OOM | TBD | TBD | TBD | TBD | TBD |
| 7. WAL corruption | TBD | TBD | TBD | TBD | TBD |
| 8. Process hang | TBD | TBD | TBD | TBD | TBD |

**目标**: 8/8 cases pass + TPC-H 22/22 维持

---

## 4. 资源需求 (W12 D3-4)

- 真实 server binary: `cargo build --release --bin sqlrustgo`
- sysbench: 已安装 (1.0.20)
- root 权限 (OOM case 需 cgroup)
- 临时磁盘: 100MB+ (per case)
- Z6G4 环境

---

## 5. 与 G8 (mock) 的关系

| 维度 | G8 (Mock) | G14 (Real) |
|------|-----------|------------|
| 工具 | harness mock | **真实 sys_kill / dd / cgroup** |
| 进程 | 进程内 mock | **真实 sqlrustgo 进程** |
| 数据 | 内存 hashmap | **真实磁盘 WAL/Page** |
| 恢复验证 | mock assert | **TPC-H 22/22 比对** |
| CI 触发 | 每次 PR | **仅 GA 前 (Z6G4)** |
| 时间 | 几秒 | 8 × 30s = 4 分钟 |

**两者互补, 不重复**: G8 验证**逻辑**, G14 验证**系统集成**.

---

## 6. 验收 (W12 D4 后)

```
✅ G8 100+ scenarios PASS (Hermes #3174)
✅ G7 Soak 10/10 PASS
⏳ G14 8/8 真实 cases PASS (W12 D3-4 Z6G4)
⏳ TPC-H 22/22 维持 (跨所有 8 cases)
```

---

**Ref**:
- V390_TEST_PLAN_ROUND2_REVIEW §G14
- scripts/crash/run_*.sh (orchestrator + 8 sub-scripts)
- scripts/gate/check_g14_real_crash.sh (门禁)
- docs/releases/v3.8.0/openspec/3174-crash-test-framework.md (G8 mock 基础)
- GATE_CONDITIONS.md GA §4.2 (崩溃恢复)
