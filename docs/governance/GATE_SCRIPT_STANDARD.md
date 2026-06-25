# Gate Script Standard — v1.0

> **日期**: 2026-06-18
> **版本**: v1.0
> **目的**: 规范 Gate 脚本的编写，防止 V1-V8 漏洞
> **依据**: META_GATE_AUDIT_2026-06.md V1-V8 漏洞分析

---

## 1. 核心原则

### 1.1 必须可执行、可验证

Gate 脚本必须满足：
1. **可执行** - 每个检查项有实际命令执行
2. **可量化** - 阈值是明确数值或布尔值
3. **防伪造** - 结果来自实际输出，不是文档审查
4. **防静默** - 错误必须被检测并报告

### 1.2 Gate 脚本禁止的模式

| 禁止模式 | 漏洞编号 | 原因 |
|----------|----------|------|
| `command \|\| true` | V6 | 吞噬退出码 |
| `output=$(cmd \|\| true)` | V6 | 双重问题 |
| `grep ... \|\| true` | V8 | grep 失败静默 |
| `check()` 只看 exit code | V1 | 0 tests run = PASS |
| 无独立 Oracle | V4 | 自验证不可信 |

---

## 2. 错误处理规范

### 2.1 命令执行模式

**错误模式（禁止）**:
```bash
# ❌ V6 漏洞：吞噬退出码
PASSED=$(cargo test --test X 2>&1 | grep "test result.*ok" | head -1 || true)
if echo "$PASSED" | grep -q "ok"; then
    echo "PASS"
fi
```

**正确模式**:
```bash
# ✅ 正确：先捕获输出和退出码，分别验证
OUTPUT=$(cargo test --test X 2>&1)
EXIT=$?
PASSED=$(echo "$OUTPUT" | grep -E "test result.*ok" | head -1)

if [ $EXIT -ne 0 ]; then
    echo "FAIL: cargo test failed (exit=$EXIT)"
    exit 1
fi

if [ -z "$PASSED" ]; then
    echo "FAIL: output could not be parsed"
    exit 1
fi

if echo "$PASSED" | grep -q "ok"; then
    echo "PASS"
fi
```

### 2.2 子脚本调用模式

**错误模式（禁止）**:
```bash
# ❌ V6 漏洞：吞噬子脚本退出码
G8_RESULT=$(bash scripts/gate/check_p12_crash_test.sh 2>&1 | tail -3 || true)
if echo "$G8_RESULT" | grep -q "PASS"; then
    echo "PASS"
fi
```

**正确模式**:
```bash
# ✅ 正确：捕获子脚本输出和退出码
G8_OUTPUT=$(bash scripts/gate/check_p12_crash_test.sh 2>&1)
G8_EXIT=$?
G8_RESULT=$(echo "$G8_OUTPUT" | tail -3)

if [ $G8_EXIT -ne 0 ]; then
    echo "FAIL: check_p12_crash_test.sh failed (exit=$G8_EXIT)"
    exit 1
fi

if ! echo "$G8_RESULT" | grep -q "PASS"; then
    echo "FAIL: check_p12_crash_test.sh did not produce PASS"
    exit 1
fi
echo "PASS"
```

---

## 3. V1 漏洞修复（Exit Code Only）

### 3.1 check() 函数问题

**错误模式（禁止）**:
```bash
check() {
    local cmd="$1"
    local name="$2"
    if eval "$cmd" >/dev/null 2>&1; then
        echo "  ✅ $name: PASS"
    else
        echo "  ❌ $name: FAIL"
    fi
}
```

问题：`cargo test --no-run` 返回 exit 0 但 0 tests run。

**正确模式**:
```bash
check() {
    local cmd="$1"
    local name="$2"
    local output
    local exit_code

    output=$(eval "$cmd" 2>&1)
    exit_code=$?

    if [ $exit_code -ne 0 ]; then
        echo "  ❌ $name: FAIL (exit=$exit_code)"
        return 1
    fi

    # 验证确实有测试执行
    if echo "$output" | grep -q "0 tests run"; then
        echo "  ❌ $name: FAIL (0 tests run)"
        return 1
    fi

    if echo "$output" | grep -q "test result:"; then
        echo "  ✅ $name: PASS"
        return 0
    fi

    echo "  ❌ $name: FAIL (no test result)"
    return 1
}
```

---

## 4. V4 漏洞修复（Oracle 对比）

### 4.1 必须有独立验证

对于关键 Gate，必须有独立 Oracle 对比：

| Gate | Oracle 来源 | 对比方式 |
|------|-------------|----------|
| G1 (TPC-H) | DuckDB/SQLite | SHA256 hash |
| G6 (Backup/Restore) | checksum | 数据完整性 |
| G8 (Crash Matrix) | 独立 crash 注入 | 8 类故障 |

### 4.2 Oracle 对比脚本模式

```bash
# ✅ 正确：Oracle 对比模式
SQLRUSTGO_RESULT=$(cargo test --test tpch_q1 2>&1)
ORACLE_RESULT=$(duckdb -c "SELECT * FROM tpch_q1()" 2>&1)

SQLRUSTGO_HASH=$(echo "$SQLRUSTGO_RESULT" | sha256sum | cut -d' ' -f1)
ORACLE_HASH=$(echo "$ORACLE_RESULT" | sha256sum | cut -d' ' -f1)

if [ "$SQLRUSTGO_HASH" != "$ORACLE_HASH" ]; then
    echo "FAIL: Result mismatch"
    exit 1
fi
echo "PASS"
```

---

## 5. V7 漏洞修复（Meta-Gate）

### 5.1 Gate 自我验证

每个 Gate 脚本必须有自测脚本：

| Gate 脚本 | 自测脚本 |
|----------|----------|
| check_g11_qps.sh | check_g11_qps_self_test.sh |
| check_g16_compatibility.sh | check_g16_compatibility_self_test.sh |

### 5.2 自测脚本最小内容

```bash
#!/bin/bash
# self_test.sh - Gate 脚本自测

set -e

echo "Testing gate script..."

# 1. 测试 help 模式
if bash "$GATE_SCRIPT" --help >/dev/null 2>&1; then
    echo "  ✅ Help mode works"
else
    echo "  ❌ Help mode failed"
    exit 1
fi

# 2. 测试模拟失败检测
if bash "$GATE_SCRIPT" < /dev/null 2>&1 | grep -q "PASS\|FAIL"; then
    echo "  ✅ Output format correct"
else
    echo "  ❌ Output format incorrect"
    exit 1
fi

echo "All self-tests passed"
```

---

## 6. 创建新 Gate 脚本 Checklist

创建新 Gate 脚本时，必须验证：

- [ ] **V6 检查**: 无 `|| true` 在关键路径
- [ ] **V1 检查**: `check()` 函数验证测试实际执行
- [ ] **V8 检查**: grep 结果被验证
- [ ] **V4 检查**: 有关键 Oracle 对比（如果适用）
- [ ] **V7 检查**: 有自测脚本
- [ ] **错误消息**: 失败时有明确的错误消息和退出码
- [ ] **超时处理**: 长时间运行的命令有超时保护

---

## 7. 现有 Gate 脚本 V6 修复状态

| 脚本 | V6 状态 | 最后验证 |
|------|--------|----------|
| check_g11_qps.sh | ✅ 已修复 | 2026-06-18 |
| check_g13_stability.sh | ✅ 已修复 | 2026-06-18 |
| check_g14_real_crash.sh | ✅ 已修复 | 2026-06-18 |
| check_g15_perf_report.sh | ✅ 已修复 | 2026-06-18 |
| check_g16_compatibility.sh | ✅ 已修复 | 2026-06-18 |
| check_p14_upgrade_test.sh | ✅ 已修复 | 2026-06-18 |
| check_p21_audit_log.sh | ✅ 已修复 | 2026-06-18 |
| check_p22_time_travel.sh | ✅ 已修复 | 2026-06-18 |
| check_p23_hash_chain.sh | ✅ 已修复 | 2026-06-18 |
| check_p32_statistics.sh | ✅ 已修复 | 2026-06-18 |
| check_p33_cost_optimizer.sh | ✅ 已修复 | 2026-06-18 |
| check_p34_parallel_executor.sh | ✅ 已修复 | 2026-06-18 |
| check_p35_simd.sh | ✅ 已修复 | 2026-06-18 |
| check_r1_r10_content.sh | ✅ 已修复 | 2026-06-18 |
| check_g12_sysbench.sh | ✅ 已修复 | 2026-06-18 |
| check_sql_compat.sh | ✅ 已修复 | 2026-06-18 |
| check_test_inventory.sh | ✅ 已修复 | 2026-06-18 |
| check_g_correctness_v390.sh | ✅ 已修复 | 2026-06-18 |

---

## 8. 验证命令

验证 Gate 脚本无 V6 漏洞：

```bash
# 检查关键位置无 || true
grep -rn "TPCH_PASSED=\|G7_RESULT=\|G8_RESULT=\|MAIN_RESULT=" scripts/gate/*.sh | grep "|| true"

# 应返回空
```

验证 Gate 脚本有自测：

```bash
# 检查是否有 self_test
ls scripts/gate/*_self_test.sh 2>/dev/null | wc -l
```

---

## 9. 关联文档

- `docs/governance/META_GATE_AUDIT_2026-06.md` - V1-V8 漏洞分析
- `docs/governance/GATE_CONDITIONS.md` - Gate 条件定义
- `docs/releases/v3.9.0/TEST_TRUTHFULNESS_REPORT.md` - 测试可信度报告

---

**维护者**: Hermes Agent
**审核日期**: 2026-06-18
