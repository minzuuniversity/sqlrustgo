# v3.8.0 Alpha Gate 门禁检查 — 执行手册

## 概述

v3.8.0 Alpha Gate 是 Evidence Graph Gate v4.1 的首次 CI 验证，目标：
- 验证 graph-cli 可执行
- 验证 evidence chain 完整性
- 验证 gate.yml workflow 正常运行

## 阶段划分

### Alpha.1 — 工具链验证

```
执行：cargo build -p graph-cli --release
验证：./target/release/gate --version
验证：./target/release/ingest --version
```

**PASS 标准**：两个 binary 均可正常执行

---

### Alpha.2 — Evidence Chain 完整性

```bash
# 1. Ingest task + commit + CI + artifact
./target/release/ingest task "v3.8.0" --branch develop/v3.8.0 --db /tmp/eg_alpha.db
./target/release/ingest commit <COMMIT_SHA> --author "openclaw" --db /tmp/eg_alpha.db
./target/release/ingest ci "alpha-check" --status PASS --db /tmp/eg_alpha.db
./target/release/ingest artifact "v3.8.0-alpha" --db /tmp/eg_alpha.db

# 2. 创建链接链
./target/release/ingest link task --task "v3.8.0" --commit <COMMIT_SHA> --db /tmp/eg_alpha.db
./target/release/ingest link ci --commit <COMMIT_SHA> --ci "alpha-check" --db /tmp/eg_alpha.db
./target/release/ingest link artifact --ci "alpha-check" --artifact "v3.8.0-alpha" --db /tmp/eg_alpha.db

# 3. Gate evaluate
./target/release/gate evaluate --task "v3.8.0" --db /tmp/eg_alpha.db
```

**PASS 标准**：`{"result":"PASS","reason":"reachability","missing":[]}`

---

### Alpha.3 — Document Gate（D1-D8）

检查项：
- D1: CHANGELOG.md 存在
- D2: CHANGELOG.md 包含 v3.8.0 条目
- D3: docs/releases/v3.8.0/VERSION_PLAN.md 存在
- D4: docs/releases/v3.8.0/DEVELOPMENT_PLAN.md 存在
- D5: docs/releases/v3.8.0/TEST_PLAN.md 存在
- D6: 无 PENDING 占位符（在 VERSION_PLAN + CHANGELOG 中）
- D7: SSOT 文档存在性（VERSION_PLAN → CHANGELOG 链接）
- D8: 无死链（VERSION_PLAN 内链接可访问）

执行：
```bash
bash scripts/gate/check_alpha_v3.8.0.sh
# 或手动检查上述 D1-D8 项
```

---

### Alpha.4 — Code Gate（C1-C6）

检查项：
- C1: `cargo build -p evidence-graph --release` 成功
- C2: `cargo test -p evidence-graph` 全部 PASS（5个测试）
- C3: `cargo build -p sqlrustgo-gate --release` 成功
- C4: 工具链 CI workflow 存在（.gitea/workflows/gate.yml）
- C5: 无编译警告（cargo build 2>&1 | grep warning）
- C6: evidence-graph GraphStats 包含 artifact_count

执行：
```bash
cargo build -p evidence-graph --release 2>&1 | grep -E 'error|warning' | head -20
cargo test -p evidence-graph 2>&1
```

---

## 快速执行命令（单行）

```bash
# Alpha 全量检查
cd /home/ai/sqlrustgo && \
cargo build -p graph-cli --release 2>&1 | tail -3 && \
cargo test -p evidence-graph 2>&1 | tail -5 && \
bash scripts/gate/check_evidence_binding.sh v3.8.0 /tmp/evidence 2>&1 | tail -10
```

## 输出格式

每项检查必须输出：
- 检查项编号（D1/C1 等）
- 状态：PASS | FAIL
- 证据：命令输出或文件内容摘要
- 如果 FAIL：具体原因 + 修复建议

## 禁止

- ❌ 跳过任何检查项
- ❌ 用历史数据冒充本次执行结果
- ❌ PENDING 占位符
- ❌ 假设通过（必须实际执行命令）