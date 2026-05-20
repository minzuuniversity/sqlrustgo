# GMP 内审检索系统 v1.0.0 部署指南

> **版本**: 1.0.0
> **更新日期**: 2026-05-21
> **维护人**: TRAE + DeepSeek-V4-Pro

---

## 1. 环境要求

### 1.1 硬件

| 组件 | 最低要求 | 推荐配置 |
|------|----------|----------|
| CPU | 8 核 | 16 核 (x86_64) |
| RAM | 16 GB | 32 GB |
| 磁盘 | 10 GB SSD | 20 GB SSD |
| GPU | 不需要 | NVIDIA RTX 5080 (可选加速) |

### 1.2 软件

| 软件 | 版本 | 说明 |
|------|------|------|
| 操作系统 | macOS 14+ / Ubuntu 22.04+ | 仅 x86_64 (当前) |
| Rust | ≥ 1.80 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Ollama | ≥ 0.1.0 | 本地 LLM 推理服务 |

---

## 2. 安装步骤

### 2.1 克隆项目

```bash
git clone http://192.168.0.252:3000/openclaw/GMP-Platform.git
cd GMP-Platform
git checkout develop/v1.0.0
```

### 2.2 编译

```bash
# Release 模式 (推荐)
cargo build --release

# 验证
./target/release/gmp-retrieve --version
# 输出: gmp-retrieve v1.0.0
```

### 2.3 安装 Ollama + 模型

```bash
# 安装 Ollama (macOS)
brew install ollama

# 启动服务
ollama serve

# 拉取模型
ollama pull bge-m3:latest              # ~2.2 GB, 嵌入模型
ollama pull bge-reranker-v2-m3:latest  # ~1.5 GB, 重排模型
ollama pull qwen2.5:14b                # ~9 GB, LLM 模型

# 验证
curl http://localhost:11434/api/tags
```

---

## 3. 数据准备

### 3.1 数据文件

确保以下目录存在且数据完整：

```bash
# GMP 文档集
ls gmp-md/*.md | wc -l          # 应为 1696

# 规则库
ls agent-knowledge/*.yaml | wc -l  # 应为 ≥ 55

# 测试场景
ls gmp_kg_build/test_results_final/391_scenarios.json
```

### 3.2 索引构建

```bash
# 全量索引构建 (耗时 2-4 小时)
gmp-retrieve index --source gmp-md/ --source agent-knowledge/

# 验证索引
gmp-retrieve index --validate
```

---

## 4. 配置

### 4.1 环境变量

```bash
# 可选配置 (导出到 .bashrc 或 .zshrc)
export OLLAMA_HOST="http://localhost:11434"
export GMP_DB_PATH="./gmp.db"
```

### 4.2 检索配置 (可选)

```bash
# 使用默认配置即可，如需自定义:
gmp-retrieve search "偏差" --config config.yaml
```

---

## 5. 验证部署

### 5.1 基本功能验证

```bash
# 1. 全文检索
gmp-retrieve search "SOP" --channel fts | head -5

# 2. 语义检索
gmp-retrieve search "偏差管理" --channel vector | head -5

# 3. 图谱检索
gmp-retrieve search "CAPA" --channel graph | head -5

# 4. 规则检索
gmp-retrieve search "偏差分级" --channel rule

# 5. 全通道
gmp-retrieve search "验证主计划" --all
```

### 5.2 LLM 功能验证

```bash
# 交互式问答
echo "偏差分级标准是什么？" | gmp-retrieve shell
```

### 5.3 性能验证

```bash
# 单查询
time gmp-retrieve search "偏差与CAPA" --all --rerank

# 批量测试
gmp-retrieve bench --rounds 20
```

---

## 6. 运维

### 6.1 数据库管理

```bash
# 备份
cp gmp.db gmp.db.bak.$(date +%Y%m%d)

# 恢复
cp gmp.db.bak.20260521 gmp.db

# 重建 (生产谨慎操作)
rm -f gmp.db && gmp-retrieve index --source gmp-md/
```

### 6.2 Ollama 管理

```bash
# 查看模型
ollama list

# 重启服务
pkill ollama && ollama serve &

# 查看日志
ollama logs
```

### 6.3 监控

```bash
# 查看 Ollama 资源占用
ps aux | grep ollama

# 查看 gmp.db 大小
ls -lh gmp.db
```

---

## 7. 故障排查

### 7.1 Ollama 连接失败

```text
错误: Connection refused (os error 61)
```

解决方法:
```bash
# 确认 Ollama 运行
ps aux | grep ollama

# 启动 Ollama
ollama serve &
```

### 7.2 索引构建内存不足

```text
错误: Cannot allocate memory
```

解决方法:
```bash
# 减少批处理大小
gmp-retrieve index --source gmp-md/ --batch-size 50
```

### 7.3 向量搜索返回空

```bash
# 验证向量索引
gmp-retrieve index --validate vector

# 如果 chunks=0，需要重建
rm -f gmp.db
gmp-retrieve index --source gmp-md/
```

---

## 8. 安全

### 8.1 文件权限

```bash
chmod 600 gmp.db
chmod 755 target/release/gmp-retrieve
```

### 8.2 网络

- Ollama 默认监听 `localhost:11434`，不对外暴露
- 如需远程访问 Ollama，修改 `OLLAMA_HOST=0.0.0.0` 并配置防火墙

### 8.3 API Key 检查

```bash
# 确保无硬编码密钥
grep -r "sk-" crates/ --include="*.rs"   # 应为空
```