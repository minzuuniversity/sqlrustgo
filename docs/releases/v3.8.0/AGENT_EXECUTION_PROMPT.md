# v3.8.0 门禁执行 — AI Agent 提示词

## 当前状态

- **版本**：v3.8.0 (develop/v3.8.0)
- **Gate**：Alpha
- **目标**：验证 Evidence Graph Gate v4.1 CI pipeline 正常运行

---

## 你需要做的（按顺序执行）

### 步骤 1：工具链编译

```bash
cd /home/ai/sqlrustgo
cargo build -p graph-cli --release
```

验证 `target/release/gate` 和 `target/release/ingest` 存在。

### 步骤 2：运行 Alpha Chain Test

```bash
cd /home/ai/sqlrustgo
bash scripts/gate/run_alpha_chain_test.sh
```

预期：`{"result":"PASS","reason":"reachability","missing":[]}`

### 步骤 3：Document Audit

检查 docs/releases/v3.8.0/ 目录：

| 文件 | 检查 |
|------|------|
| VERSION_PLAN.md | 存在、无 PENDING |
| DEVELOPMENT_PLAN.md | 存在、无 PENDING |
| TEST_PLAN.md | 存在、无 PENDING |

执行：
```bash
cd /home/ai/sqlrustgo
for f in docs/releases/v3.8.0/*.md; do
  echo "=== $f ==="
  wc -l "$f"
  grep -c "PENDING\|TODO" "$f" || echo "0 pending"
done
```

### 步骤 4：Code Gate

```bash
cd /home/ai/sqlrustgo
cargo test -p evidence-graph 2>&1 | tail -15
cargo build -p evidence-graph --release 2>&1 | tail -3
```

验证 5/5 tests PASS。

### 步骤 5：文档质量扫描

执行：
```bash
cd /home/ai/sqlrustgo
bash scripts/gate/check_evidence_binding.sh v3.8.0 /tmp/evidence_check 2>&1
bash scripts/gate/check_ssot_duplicate.sh 2>&1
```

---

## 输出要求

每项检查输出：

```
[✓/✗] CHECK_NAME
  Evidence: <command output or file content>
```

如果 FAIL，立即停止并报告。

---

## 禁止

- PENDING 占位符
- 假设通过
- 历史数据冒充
- 跳过检查项