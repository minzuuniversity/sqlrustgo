# GMP 内审检索系统 版本生命周期

> **版本**: 1.0
> **创建日期**: 2026-05-21
> **维护人**: TRAE + DeepSeek-V4-Pro
> **适用项目**: GMP 内审检索系统 (gmp-retrieve)

> **SSOT 声明**: 门禁定义以 `GATE_SPEC.md` 为唯一权威。

---

## 一、版本阶段模型

GMP 内审检索系统使用四级门禁发布生命周期：

```
A-Gate → B-Gate → R-Gate → G-Gate
 (α入口)  (β入口)  (RC入口)  (GA入口)
```

该模型适配 CLI 工具 + 数据库项目的开发环境，确保每个发布阶段的质量门槛。

---

## 二、阶段详解

### 2.1 A-Gate (Alpha Gate)

**阶段说明**: 核心功能开发完成，系统可运行验证

**特点**:
- 四层检索管线可运行（Layer 1 + Layer 2）
- 数据导入完成（1696 文档入库）
- 四通道检索可用（规则/向量/全文/图谱）
- RRF 融合可用

**门禁要求**:
| 检查项 | 通过标准 |
|--------|----------|
| 编译 | `cargo build --release` 无错误 |
| 单元测试 | `cargo test` ≥ 80% 通过 |
| 数据完整性 | 1696 文档全部入库 |
| 索引可用 | FTS/向量/图索引可查 |
| RRF 管线 | 四通道融合返回结果 |

**产出**:
- `gmp-retrieve` 二进制
- 数据底座就绪的 SQLRustGo 数据库

---

### 2.2 B-Gate (Beta Gate)

**阶段说明**: 功能冻结，检索管线稳定

**特点**:
- Layer 3 Reranker 集成
- Layer 4 LLM 问答集成
- CLI 全命令可用
- 391 场景初步测试基线

**门禁要求**:
| 检查项 | 通过标准 |
|--------|----------|
| 编译 | `cargo build --release` 无错误 |
| 单元测试 | `cargo test` ≥ 90% 通过 |
| 功能测试 | CLI 所有子命令可用 |
| 391 场景 | 通过率 ≥ 65% |
| 检索基准 | recall@20 ≥ 70% |

**产出**:
- 完整四层检索管线
- 391 场景测试基线

---

### 2.3 R-Gate (RC Gate)

**阶段说明**: 发布候选，全面验证

**特点**:
- 391 场景通过率验证
- 失败模式分析完成
- 性能基准建立
- 文档齐全

**门禁要求**:
| 检查项 | 通过标准 |
|--------|----------|
| 391 场景 | 通过率 ≥ 80% |
| 性能 | P50 < 5s, P95 < 15s |
| 安全 | API Key 不泄露, 无高危依赖 |
| 文档 | 用户手册 + 部署指南齐全 |
| 稳定性 | 连续 50 次查询无崩溃 |

**产出**:
- 完整测试报告
- 对比基线报告
- 最终交付文档

---

### 2.4 G-Gate (GA Gate)

**阶段说明**: 正式发布

**特点**:
- 所有门禁通过
- 发布说明完成
- Git Tag: v1.0.0

**门禁要求**:
| 检查项 | 通过标准 |
|--------|----------|
| 全部 RC 检查 | PASS |
| 文档完整性 | 必选文档齐全 |
| 版本一致性 | 版本号统一 |
| 发布 Tag | v1.0.0 |

**产出**:
- Release v1.0.0
- 发布公告

---

## 三、分支与 Tag 规范

### 3.1 分支命名

| 分支类型 | 命名规则 | 示例 |
|----------|----------|------|
| 开发分支 | `develop/vX.Y.Z` | `develop/v1.0.0` |
| Alpha | `alpha/vX.Y.Z` | `alpha/v1.0.0` |
| Beta | `beta/vX.Y.Z` | `beta/v1.0.0` |
| RC | `rc/vX.Y.Z` | `rc/v1.0.0` |
| 发布分支 | `release/vX.Y.Z` | `release/v1.0.0` |

### 3.2 Tag 命名

```text
v1.0.0-beta1    # Beta 阶段
v1.0.0-rc1      # RC 阶段
v1.0.0          # GA 版本
```

### 3.3 Issue 标签

| 标签 | 用途 |
|------|------|
| `type/feature` | 新功能 |
| `type/bugfix` | Bug 修复 |
| `type/docs` | 文档 |
| `priority/p0` | 阻塞性问题 |
| `priority/p1` | 高优先级 |
| `priority/p2` | 一般优先级 |
| `source/gate-alpha` | Alpha 门禁相关问题 |
| `source/gate-beta` | Beta 门禁相关问题 |
| `source/gate-rc` | RC 门禁相关问题 |

---

## 四、阶段转换流程

### 4.1 Alpha → Beta

```
step 1: Alpha Gate 全部检查 PASS
step 2: 创建 ISSUE 追踪未完成的 P1/P2 任务
step 3: 打 Tag: v1.0.0-beta1
step 4: 创建分支: beta/v1.0.0
```

### 4.2 Beta → RC

```
step 1: Beta Gate 全部检查 PASS
step 2: 功能冻结：不再接受新功能请求
step 3: 打 Tag: v1.0.0-rc1
step 4: 创建分支: rc/v1.0.0
step 5: 进入 bug fix only 模式
```

### 4.3 RC → GA

```
step 1: RC Gate 全部检查 PASS
step 2: 391 场景通过率 ≥ 80%
step 3: 文档全部完成
step 4: 创建 release/v1.0.0 分支
step 5: 打 release Tag: v1.0.0
step 6: 发布 Gitea Release
```