# GMP-Platform v1.2.0 测试体系设计

> **版本**: 1.2.0
> **更新日期**: 2026-05-24
> **PR**: #108 (Dashboard full integration)
> **Gate**: L0-Alpha/Beta/RC/GA

---

## 一、测试分层架构

```
┌─────────────────────────────────────────────────┐
│           408 场景测试 (手动/CI)                 │  ← 真实 GMP 场景，端到端
├─────────────────────────────────────────────────┤
│           集成测试 (tests/)                      │  ← 多 crate 协作
├─────────────────────────────────────────────────┤
│           单元测试 (#[test])                    │  ← 单个函数/模块
├─────────────────────────────────────────────────┤
│           性能压测 (scripts/stress_test_72h.py)  │  ← 72h 连续压测
└─────────────────────────────────────────────────┘
```

### 与 v1.1.0 差异

| 层级 | v1.1.0 | v1.2.0 变更 |
|------|--------|-------------|
| 存储层 | rusqlite | SQLRustGo FileStorage + InvertedIndex |
| 检索 | BM25 FTS | BM25 + partial-match + LIKE n-gram fallback |
| Web Dashboard | 无 | dashboard/server.py + index.html |
| 移动端 | 本地目录 | gmp_agent/ 纳入版本管理 |
| 压测 | 短时测试 | 72h 连续压测脚本 |

---

## 二、单元测试

### 2.1 测试范围

| Crate | 测试文件 | 覆盖范围 |
|-------|---------|---------|
| gmp-storage | `src/tests.rs` | StorageEngine, SqlrustgoAdapter |
| gmp-retrieval | `src/tests.rs` | Channel engine, RRF fusion |
| gmp-api | `src/tests.rs` | REST handlers |

### 2.2 运行命令

```bash
cargo test --lib
cargo test --all-features
```

### 2.3 通过标准

- RC Gate: 100% 通过
- Beta Gate: ≥95% 通过

---

## 三、集成测试

### 3.1 E2E 检索测试

四通道检索全可用：

```bash
./target/release/gmp-retrieve search --channel fts --db ./data/gmp.db -- '偏差处理'
./target/release/gmp-retrieve search --channel rule --db ./data/gmp.db -- '偏差处理'
./target/release/gmp-retrieve search --channel vector --db ./data/gmp.db -- '偏差处理'
./target/release/gmp-retrieve search --channel graph --db ./data/gmp.db -- '偏差处理'
```

### 3.2 Dashboard API 测试

```bash
python3 dashboard/server.py &
curl http://localhost:8080/api/stats
curl http://localhost:8080/api/audit
```

---

## 四、场景测试（408 场景）

### 4.1 测试数据

- 位置: `scripts/eval/test_sets/gmp_scenarios_v2.json`
- 场景数: 408 个
- 覆盖: 变更控制、数据完整性、验证生命周期、偏差处理等

### 4.2 通过标准

| Gate | 标准 |
|------|------|
| Beta | ≥65% |
| RC | ≥90% |
| GA | ≥95% |

### 4.3 运行命令

```bash
cd scripts/eval
python3 eval_hybrid_search.py --scenarios gmp_scenarios_v2.json --enable-vector
```

---

## 五、72h 压测

### 5.1 测试场景

| 场景 | 说明 | 测试频率 |
|------|------|---------|
| normal_weighing | 正常称量流程 | 50% |
| over_limit_capa | 超限/超容量 | 30% |
| invalid_blocked | 无效/被封锁 | 20% |

### 5.2 压测指标

| 指标 | RC 标准 | GA 标准 |
|------|---------|---------|
| 通过率 | ≥90% | ≥95% |
| P99 延迟 | <15000ms | <10000ms |
| Ollama 可用率 | ≥99% | ≥99.9% |
| 数据库大小 | <100MB | <100MB |

### 5.3 运行命令

```bash
# 干跑（快速验证）
python3 scripts/stress_test_72h.py --dry-run

# 完整 72h 压测
python3 scripts/stress_test_72h.py --hours 72
```

---

## 六、测试计划

### 6.1 RC 前测试

1. `cargo build --release --workspace` — 全量编译
2. `cargo test --all-features` — 全部单元测试
3. `cargo clippy --all-targets -- -D warnings` — 静态分析
4. `cargo fmt --all -- --check` — 格式检查
5. E2E 四通道检索测试
6. Dashboard API 测试
7. 72h 压测 dry-run

### 6.2 GA 前测试

1. 完整 72h 压测运行
2. 408 场景 ≥95% 通过率验证
3. DB 大小稳定性验证
4. 内存无泄漏验证
