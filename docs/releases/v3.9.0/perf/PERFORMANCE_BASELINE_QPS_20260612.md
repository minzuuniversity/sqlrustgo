# SQLRustGo v3.9.0 QPS/TPS Performance Baseline (2026-06-12)

> **Generated**: 2026-06-12
> **Ref**: Issue #3224 (Z6G4 真实 perf 测量 + 填 PERFORMANCE_BASELINE.md)
> **Status**: **Local dev baseline** (Mac mini M2) — pending Z6G4 真实 runs
> **Note**: Numbers from cargo bench (criterion) — NOT sysbench OLTP.
> **Commit**: `e34eb81ad`

---

## 1. Measurement Environment

| Item | Value | Note |
|------|-------|------|
| **Platform** | darwin arm64 (Mac mini M2) | NOT Z6G4 |
| **CPU** | Apple M2 (8 cores) | |
| **RAM** | 24 GB | |
| **OS** | macOS 26.5.1 | |
| **Tool** | cargo bench (criterion 0.5) | |
| **Settings** | warm-up=1s, measurement=3s, sample-size=10 | quick mode for local dev |
| **Storage engine** | MemoryStorage | in-process |
| **Command** | `nice -n 19 cargo bench --bench qps_bench` | |

## 2. QPS Results (Real Measurements)

### Point Select (1-16 threads)
| Threads | Throughput | Per-thread |
|---------|-----------|-----------|
| 1 | 2.95 Kelem/s | 2950 elem/s |
| 4 | 9.37 Kelem/s | 2343 elem/s |
| 8 | 11.52 Kelem/s | 1440 elem/s |
| 16 | 12.67 Kelem/s | 792 elem/s |

### Range Select (1-8 threads)
| Threads | Throughput | Per-thread |
|---------|-----------|-----------|
| 1 | 2.96 Kelem/s | 2963 elem/s |
| 4 | 10.23 Kelem/s | 2558 elem/s |
| 8 | 12.09 Kelem/s | 1511 elem/s |

### Insert (1-8 threads)
| Threads | Throughput | Per-thread |
|---------|-----------|-----------|
| 1 | 8.14 Melem/s | 8.14M elem/s |
| 4 | 5.55 Melem/s | 1.39M elem/s |
| 8 | 3.03 Melem/s | 379K elem/s |

### Update (1-8 threads)
| Threads | Throughput | Per-thread |
|---------|-----------|-----------|
| 1 | 78.0 Kelem/s | 78.0K elem/s |
| 4 | 68.4 Kelem/s | 17.1K elem/s |
| 8 | 67.0 Kelem/s | 8.4K elem/s |

## 3. Scaling Analysis

| Workload | 1→8 threads | Scaling |
|----------|-------------|---------|
| Point Select | 2.95K → 11.52K | 3.9x (sublinear) |
| Range Select | 2.96K → 12.09K | 4.1x (sublinear) |
| Insert | 8.14M → 3.03M | 0.37x (lock contention) |
| Update | 78K → 67K | 0.86x (lock contention) |

**Key findings**:
- Read workloads scale well (3-4x with 8 threads)
- Write workloads have contention at >1 thread (MemoryStorage uses RwLock)
- 16 threads point_select plateaus at 12.67K — likely RwLock contention

## 4. Comparison vs v3.8.0-rc1 Baseline

> **TODO**: Compare against `PERFORMANCE_BASELINE.md` formal table once Z6G4 measurements are available.

## 5. Verification

To reproduce:

```bash
nice -n 19 cargo bench --bench qps_bench -- \
    --warm-up-time=1 --measurement-time=3 --sample-size=10
```

## 6. Pending Work

- [ ] Re-run on Z6G4 (x86_64 8C/64GB) for formal baseline
- [ ] Compare against v3.8.0-rc1 baseline (regression detection)
- [ ] Add sysbench OLTP numbers (currently only criterion micro-bench)
