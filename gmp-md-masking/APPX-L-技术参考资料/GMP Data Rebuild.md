---
entity_type: pattern
confidence: high
domains: [sqlrustgo, gmp, workflow]
created: 2025-04-17
updated: 2025-04-17
---

# GMP Data Rebuild

## 何时需要重建

- `import_sops.py` 修复了 bug（标题解析、properties）后
- `persist_sqlite.rs` 修改了 schema 后
- 数据损坏或 schema 不一致

## 完整重建流程

### Step 1: 清理数据目录

```bash
cd /home/ai/gmp_kg_build
rm -rf ~/gmp-qa && mkdir -p ~/gmp-qa
```

### Step 2: 导入文档

```bash
export SQLRUSTGO_HOME=~/gmp-qa
python3 import_sops.py --home ~/gmp-qa
# 预计 25-30 秒，557 文档
```

### Step 3: 构建图谱

```bash
python3 build_gmp_graph.py --home ~/gmp-qa
# 预计 5-10 秒，559 节点 + 1842 边
```

### Step 4: 验证

```bash
cd /home/ai/sqlrustgo

# Ping 测试
(echo '{"cmd":"Ping"}'; sleep 3) | stdbuf -oL -eL env SQLRUSTGO_HOME=~/gmp-qa ./target/debug/sqlrustgo-gmp-cli

# SqlSearch 过滤验证
(echo '{"cmd":"SqlSearch","query":"验证","chapter":"01","top_k":5}'; sleep 3) | stdbuf -oL -eL env SQLRUSTGO_HOME=~/gmp-qa ./target/debug/sqlrustgo-gmp-cli

# GraphNeighbors 验证
(echo '{"cmd":"GraphNeighbors","node_type":"Department","node_name":"QA","top_k":5}'; sleep 3) | stdbuf -oL -eL env SQLRUSTGO_HOME=~/gmp-qa ./target/debug/sqlrustgo-gmp-cli
```

## import_sops.py 修复历史

### Bug 1: 434 个文档标题为 `\`\`\`markdown`

**根因**：第一行非空行是代码围栏 ````markdown`
**修复**：跳过 `\`\`\`` 开头的行，优先解析 YAML frontmatter

### Bug 2: properties 全为空 `{}`

**根因**：Python 端从未传入 properties，Rust 端 save_document 漏写 properties 列
**修复**：
1. Python: 解析 frontmatter，提取 department/category，传入 properties
2. Rust: INSERT 语句补上 properties 列

## 性能数据

| 操作 | 数据量 | 耗时 |
|------|--------|------|
| import_sops.py | 557 docs | 30s |
| build_gmp_graph.py | 559 节点 + 1842 边 | 22s |
| CLI 冷启动 + reload | 557 docs | ~10s |

## 相关

- [[SQLRustGo GMP CLI]]
- [[Persist Sqlite Backend]]
