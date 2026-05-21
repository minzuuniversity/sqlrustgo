# GMP-Platform Issue 12-20 实现计划

## 现状分析

### Issue #12 LLM 问答 + 意图分类器
- **状态**: ✅ **已完成**
- **证据**: `crates/gmp-llm/src/lib.rs` (204行) 包含完整的 `LlmClient` 实现：
  - `classify_intent()` - 意图分类 (Rule/Factual/Procedural/Clarification/Comparison/General)
  - `route_query()` - 基于意图路由到四通道
  - `generate_answer()` - RAG 答案生成
  - `chat()` - Ollama Chat Completion
- **结论**: 标记完成，无需开发

### Issue #13 CLI 全命令 (index/search/evaluate/shell/bench)
- **状态**: ⚠️ **部分实现**
- **证据**:
  - `Index` ✅ 完整实现（index_documents + validate）
  - `Search` ✅ 完整实现（四通道搜索）
  - `Evaluate` ❌ placeholder (仅打印 "placeholder")
  - `Shell` ✅ 基本实现（交互式循环 + search/index 命令）
  - `Bench` ❌ placeholder (仅打印 "placeholder")
- **待开发**: Evaluate 真实评测逻辑 + Bench 真实基准测试

### Issue #14 391 场景测试 + 通过率报告
- **状态**: ❌ **未开始**
- **要求**: M5 优先级，391 个 GMP 检查点场景测试
- **前置**: 需要 gmp-retrieval + gmp-storage 功能完整

### Issue #15 用户手册 + 部署指南 + 发布文档
- **状态**: ⚠️ **部分完成**
- **证据**: `docs/user/USER_MANUAL.md` 已存在
- **待完成**: 部署指南 + 发布文档

### Issue #16 索引增量更新支持
- **状态**: ❓ **需调研**
- **证据**: storage.rs 中有 `updated_at` 字段，但无增量更新逻辑
- **待开发**: 增量索引更新机制

### Issue #17 性能压测 + 基准报告
- **状态**: ❌ **未开始**
- **前置**: Issue 13 的 Bench 命令先完成

### Issue #18 中国 GMP 条文补充到知识库
- **状态**: ❌ **未开始**
- **性质**: 数据准备任务，非代码开发

### Issue #19 设备手册数据集成
- **状态**: ❌ **未开始**
- **前置**: Issue 9 数据导入基础设施已完成

### Issue #20 Web API 模式 (RESTful)
- **状态**: ❌ **未开始**
- **性质**: 新建 Web API 层

---

## 实现计划 (按依赖排序)

### Phase 1: Issue 13 CLI 完善 (Evaluate + Bench)
**预计工作量**: 2h

1. **Evaluate 命令** - 实现评测逻辑
   - 读取测试集 JSON 文件
   - 对每个 query 调用四通道检索
   - 计算 recall@K / MRR / NDCG
   - 输出格式化报告

2. **Bench 命令** - 实现性能基准测试
   - 并发查询测试
   - 吞吐量 (QPS) 测量
   - 延迟分布 (p50/p95/p99)
   - 结果写入 JSON 报告

### Phase 2: Issue 14 391 场景测试
**预计工作量**: 4h

3. **391 测试场景文件** - 准备 GMP 391 检查点测试集
   - 按 GMP 章节组织
   - 每条包含: query + 期望文档ID + 关键词

4. **Evaluate 集成** - 批量运行 + 报告生成
   - 计算通过率
   - 输出 PASS/FAIL 报告

### Phase 3: Issue 15 文档完善
**预计工作量**: 2h

5. **部署指南** - `docs/user/DEPLOYMENT.md`
   - Ollama 安装配置
   - 向量模型下载
   - 数据库初始化
   - 常见问题

6. **发布文档** - `docs/releases/v1.0.0/`
   - Release notes
   - 迁移指南

### Phase 4: Issue 16 增量索引
**预计工作量**: 3h

7. **增量更新机制**
   - 监听文件变化
   - 新文档增量入库
   - 已有文档更新重索引

### Phase 5: Issue 17 性能压测
**预计工作量**: 2h

8. **压测 + 基准报告**
   - 结合 Bench 命令输出
   - 生成 HTML 基准报告

### Phase 6: Issue 18-20
**说明**: 数据准备和 API 层，优先级 P2-P2，可后续迭代

---

## 开发分支策略

```
develop/v1.0.0 (protected)
├── feature/13-cli-evaluate-bench  (Issue 13)
├── feature/14-391-scenarios       (Issue 14)
├── feature/15-docs                (Issue 15)
├── feature/16-incremental-index   (Issue 16)
└── feature/17-benchmark-report    (Issue 17)
```

每个 Issue 完成后立即提交 PR 合并到 develop/v1.0.0，再继续下一个。

---

## 执行顺序

1. Issue 13 (CLI) → PR → close #13
2. Issue 14 (391测试) → PR → close #14
3. Issue 15 (文档) → PR → close #15
4. Issue 16 (增量索引) → PR → close #16
5. Issue 17 (压测) → PR → close #17
6. Issue 18-20 (P2, 后续迭代)
