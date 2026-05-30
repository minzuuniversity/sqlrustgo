# SQLRustGo 门禁检查 — 执行提示词

## 用途

将此文档内容作为 prompt 提供给其他 AI agent 或 CLI runner，执行 v3.8.0 各阶段门禁检查。

---

## 阶段 1：Alpha Gate

### A1.1 工具链编译

```bash
cd /home/ai/sqlrustgo
cargo build -p graph-cli --release 2>&1
./target/release/gate --version
./target/release/ingest --version
```

**PASS**：无编译错误，binary 可执行

### A1.2 Evidence Chain

```bash
cd /home/ai/sqlrustgo
DB=/tmp/eg_alpha_$(date +%s).db

# Ingest
./target/release/ingest task "v3.8.0-alpha" --branch develop/v3.8.0 --db $DB
./target/release/ingest commit $(git rev-parse HEAD) --author "openclaw" --db $DB
./target/release/ingest ci "alpha-check-$(date +%s)" --status PASS --db $DB
./target/release/ingest artifact "v3.8.0-alpha-artifact" --db $DB

# Link
./target/release/ingest link task --task "v3.8.0-alpha" --commit $(git rev-parse HEAD) --db $DB
./target/release/ingest link ci --commit $(git rev-parse HEAD) --ci "alpha-check-$(date +%s)" --db $DB
./target/release/ingest link artifact --ci "alpha-check-$(date +%s)" --artifact "v3.8.0-alpha-artifact" --db $DB

# Evaluate
./target/release/gate evaluate --task "v3.8.0-alpha" --db $DB
```

**PASS**：`{"result":"PASS","reason":"reachability","missing":[]}`

### A1.3 Document Gate

检查以下文件存在性 + 内容质量：

| ID | 文件 | 检查项 |
|----|------|--------|
| D1 | CHANGELOG.md | 存在 |
| D2 | CHANGELOG.md | 包含 v3.8.0 条目 |
| D3 | docs/releases/v3.8.0/VERSION_PLAN.md | 存在 |
| D4 | docs/releases/v3.8.0/DEVELOPMENT_PLAN.md | 存在 |
| D5 | docs/releases/v3.8.0/TEST_PLAN.md | 存在 |
| D6 | CHANGELOG.md + VERSION_PLAN.md | 无 PENDING 占位符 |
| D7 | docs/releases/v3.8.0/VERSION_PLAN.md | 包含指向 CHANGELOG 的链接 |
| D8 | docs/releases/v3.8.0/VERSION_PLAN.md | 内链可访问（无 404） |

执行：
```bash
# 检查 D1-D5 存在性
for f in CHANGELOG.md docs/releases/v3.8.0/VERSION_PLAN.md docs/releases/v3.8.0/DEVELOPMENT_PLAN.md docs/releases/v3.8.0/TEST_PLAN.md; do
  if [ -f "$f" ]; then echo "EXISTS: $f"; else echo "MISSING: $f"; fi
done

# 检查 D6 PENDING
grep -r "PENDING\|TODO\|FIXME" docs/releases/v3.8.0/VERSION_PLAN.md CHANGELOG.md 2>/dev/null && echo "FOUND PENDING" || echo "NO PENDING"

# 检查 D7 链接
grep -o '\[.*\](.*CHANGELOG.*)' docs/releases/v3.8.0/VERSION_PLAN.md | head -3
```

### A1.4 Code Gate

```bash
cd /home/ai/sqlrustgo

# C1: evidence-graph builds
cargo build -p evidence-graph --release 2>&1 | tail -3

# C2: evidence-graph tests
cargo test -p evidence-graph 2>&1 | tail -10

# C3: sqlrustgo-gate builds
cargo build -p sqlrustgo-gate --release 2>&1 | tail -3

# C4: workflow exists
[ -f .gitea/workflows/gate.yml ] && echo "EXISTS: gate.yml" || echo "MISSING: gate.yml"

# C5: no compilation warnings (only errors)
cargo build -p evidence-graph --release 2>&1 | grep -E "^error" | wc -l

# C6: artifact_count in GraphStats
grep -c "artifact_count" crates/evidence-graph/src/lib.rs
```

---

## 阶段 2：Beta Gate

在 Alpha PASS 基础上执行：

### B2.1 文档完整性升级

| ID | 检查项 |
|----|--------|
| B1 | VERSION_PLAN.md 包含完整的 API 变更列表 |
| B2 | DEVELOPMENT_PLAN.md 包含里程碑日期 |
| B3 | TEST_PLAN.md 包含 test case 列表 |
| B4 | 所有文档之间链接一致（无断链） |
| B5 | CHANGELOG.md 包含 breaking changes 说明（若有） |

### B2.2 功能测试

```bash
cd /home/ai/sqlrustgo

# 所有 evidence-graph 测试
cargo test -p evidence-graph -- --nocapture 2>&1

# 集成测试（如果存在）
cargo test -p sqlrustgo-gate 2>&1 | tail -5
```

### B2.3 Evidence Graph 完整性

```bash
# 读取 /tmp/eg_alpha_*.db 中最近的 DB，执行完整性检查
DB=$(ls -t /tmp/eg_alpha_*.db 2>/dev/null | head -1)
if [ -n "$DB" ]; then
  ./target/release/gate evaluate --task "v3.8.0-alpha" --db $DB
  ./target/release/gate status --db $DB
fi
```

---

## 阶段 3：RC Gate

在 Beta PASS 基础上执行：

### C3.1 版本标记检查

```bash
cd /home/ai/sqlrustgo

# 版本号一致性
grep "version" Cargo.toml | head -2
git describe --tags --always

# CHANGELOG.md 包含 RC 标记
grep -c "v3.8.0" CHANGELOG.md
```

### C3.2 安全扫描

```bash
cd /home/ai/sqlrustgo

# 依赖审计
cargo audit 2>&1 | tail -10

# 许可证检查
cargo deny check license 2>&1 | tail -10
```

### C3.3 性能基准（可选）

```bash
cd /home/ai/sqlrustgo
cargo build --release 2>&1 | tail -3

# TPC-H SF=1 quick bench
./target/release/sqlrustgo-bench-cli tpch-bench \
  --ddl scripts/tpch/tpch_schema.sql \
  --data ~/sqlrustgo-tpch-sf1/data \
  --queries all 2>&1 | tail -20
```

---

## 阶段 4：GA Gate

在 RC PASS 基础上执行：

### D4.1 门禁报告生成

```bash
cd /home/ai/sqlrustgo

# 生成最终报告
echo "## v3.8.0 GA Gate Report" > /tmp/GA_GATE_REPORT.md
echo "Date: $(date)" >> /tmp/GA_GATE_REPORT.md
echo "Commit: $(git rev-parse HEAD)" >> /tmp/GA_GATE_REPORT.md

# Alpha/Beta/RC/GA 各阶段结果汇总
echo "## Results" >> /tmp/GA_GATE_REPORT.md
echo "| Phase | Status | Evidence |" >> /tmp/GA_GATE_REPORT.md

# 发布到 docs/releases/v3.8.0/GA_GATE_REPORT.md
```

### D4.2 Git Tag

```bash
cd /home/ai/sqlrustgo
git tag -a v3.8.0-ga -m "v3.8.0 GA — Execution Architecture Consolidation"
git push gitea v3.8.0-ga
```

### D4.3 最终检查清单

| ID | 检查项 |
|----|--------|
| GA1 | 所有 4 个阶段门禁 PASS |
| GA2 | CHANGELOG.md 已更新 |
| GA3 | Git tag 已创建并推送 |
| GA4 | 文档无 PENDING 占位符 |
| GA5 | 无已知 critical bug |

---

## 执行原则

1. **必须实际执行命令**，禁止假设通过
2. **每项检查必须输出证据**（命令输出摘要）
3. **禁止 PENDING 占位**——如果某项未执行，标记为 SKIP 并说明原因
4. **Truthfulness 优先**——发现 FAIL 立即报告，不美化

## 输出格式

```
## [Phase] Gate Results

| ID | Check | Status | Evidence |
|----|-------|--------|----------|
| A1.1 | graph-cli builds | PASS | "Finished release profile" |
| A1.2 | evidence chain | PASS | "result:PASS, missing:[]" |
| D1 | CHANGELOG exists | FAIL | "file not found" |
...
```

如果任何项为 FAIL，立即停止并报告，等待人工确认。