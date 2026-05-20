# v1.0.0 Gitea ISSUE 追踪文件

> **版本**: 1.0.0
> **创建状态**: ✅ 已全部创建 (2026-05-21)
> **自动化脚本**: `scripts/create_gitea_issues.py`

---

## 一、ISSUE 创建清单

以下 ISSUE 需在 Gitea 仓库中创建。创建后填写 URL 链接。

### Phase 1: M1 项目启动 (P0)

| # | 标题 | Labels | Milestone | URL |
|---|------|--------|-----------|-----|
| [#8](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/8) | [M1] 项目脚手架 + Cargo workspace + CLI 框架 | type/feature, priority/p0 | v1.0.0-A | ✅ |
| [#9](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/9) | [M2] 数据导入 + 索引基础设施 (FTS/向量/图谱) | type/feature, priority/p0 | v1.0.0-B | ✅ |
| [#10](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/10) | [M3] 四通道检索引擎 (规则/向量/全文/图谱) | type/feature, priority/p0 | v1.0.0-B | ✅ |
| [#11](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/11) | [M4] RRF 融合 + Reranker 重排引擎 | type/feature, priority/p0 | v1.0.0-B | ✅ |
| [#12](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/12) | [M4] LLM 问答集成 + 意图分类器 | type/feature, priority/p0 | v1.0.0-RC | ✅ |
| [#13](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/13) | [M3] CLI 全命令 (index/search/evaluate/shell/bench) | type/feature, priority/p0 | v1.0.0-RC | ✅ |
| [#14](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/14) | [M5] 391 场景测试 + 通过率报告 | type/test, priority/p0 | v1.0.0-RC | ✅ |
| [#15](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/15) | [M5] 用户手册 + 部署指南 | type/docs, priority/p0 | v1.0.0-GA | ✅ |

### Phase 2: P1 增强 (v1.0.0-GA)

| # | 标题 | Labels | Milestone | URL |
|---|------|--------|-----------|-----|
| [#16](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/16) | [P1] 索引增量更新支持 | type/feature, priority/p1 | v1.0.0-GA | ✅ |
| [#17](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/17) | [P1] 性能压测 + 基准报告 | type/test, priority/p1 | v1.0.0-GA | ✅ |

### Phase 3: P2 后续 (v1.1.0+)

| # | 标题 | Labels | Milestone | URL |
|---|------|--------|-----------|-----|
| [#18](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/18) | [P2] 中国 GMP 条文补充到知识库 | type/feature, priority/p2 | v1.1.0 | ✅ |
| [#19](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/19) | [P2] 设备手册数据集成 | type/feature, priority/p2 | v1.1.0 | ✅ |
| [#20](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/20) | [P2] Web API 模式 (RESTful) | type/feature, priority/p2 | v1.1.0 | ✅ |

---

## 二、ISSUE 创建命令

在 Gitea 仓库目录下执行：

```bash
# 使用 Gitea API 创建 ISSUE (需要 token)
GITEA_TOKEN="your_token"
GITEA_URL="http://192.168.0.252:3000/api/v1"
REPO="openclaw/GMP-Platform"

create_issue() {
  local title="$1"
  local labels="$2"
  local milestone="$3"
  
  curl -s -X POST "$GITEA_URL/repos/$REPO/issues" \
    -H "Authorization: token $GITEA_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
      \"title\": \"$title\",
      \"body\": \"See docs/releases/v1.0.0/DEVELOPMENT_PLAN.md\",
      \"labels\": [$(echo $labels | sed 's/,/\",\"/g' | sed 's/^/\"/;s/$/\"/')],
      \"milestone\": $milestone
    }"
}

# P0
create_issue "[M1] 项目脚手架 + Cargo workspace + CLI 框架" "type/feature,priority/p0" 1
create_issue "[M2] 数据导入 + 索引基础设施" "type/feature,priority/p0" 1
```

### 替代方式：Gitea Web UI

如果 API 不可用，在 Gitea Web UI 手动创建 ISSUE：
1. 访问 http://192.168.0.252:3000/openclaw/GMP-Platform/issues
2. 点击 "New Issue"
3. 填写标题和标签
4. 选择 Milestone

---

## 三、Milestone 定义

| Milestone | 名称 | 截止日期 | ISSUE |
|-----------|------|----------|-------|
| v1.0.0-A | Alpha 阶段 | Week 2 D14 | [#8](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/8) |
| v1.0.0-B | Beta 阶段 | Week 3 D21 | [#9](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/9), [#10](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/10), [#11](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/11) |
| v1.0.0-RC | RC 阶段 | Week 4 D25 | [#12](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/12), [#13](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/13), [#14](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/14) |
| v1.0.0-GA | GA 发布 | Week 4 D28 | [#15](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/15), [#16](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/16), [#17](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/17) |
| v1.1.0 | 后续版本 | TBD | [#18](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/18), [#19](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/19), [#20](http://192.168.0.252:3000/openclaw/GMP-Platform/issues/20) |

---

## 四、ISSUE 模板

### 功能 ISSUE 模板

```markdown
## 概述
{简短描述}

## 验收条件
- [ ] {条件1}
- [ ] {条件2}

## 技术设计
参考: docs/releases/v1.0.0/DEVELOPMENT_PLAN.md

## 关联 PR
- PR #{N}

## 门禁关联
- A-Gate / B-Gate / R-Gate 检查项 {N}
```

### Bug ISSUE 模板

```markdown
## 问题描述
{描述}

## 复现步骤
1. 
2. 

## 预期行为
{描述}

## 实际行为
{描述}

## 环境
- OS: 
- Rust: 
- gmp-retrieve version: 
```