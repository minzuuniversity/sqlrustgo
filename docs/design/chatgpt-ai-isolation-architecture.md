# ChatGPT 隔离架构设计：AI-Native GMP 平台的数据安全

> 来源：ChatGPT 对话
> 日期：2026-05-20
> 状态：架构设计

---

## 一、核心矛盾

GMP/21 CFR Part 11/Annex 11 要求：数据最小暴露、审计可追踪、可验证、不可篡改、企业知识产权保护、患者/工艺/配方/批记录不能外泄。

而开发 HermesAgent / OpenClaw 驱动的 GMP-OS 时，又必须依赖 Claude/GPT/Gemini/DeepSeek 等云端大模型。

核心问题：**如何让云模型参与开发，但永远接触不到真正的 GMP 核心数据。**

这不是"禁用云模型"，而是**建立数据分级与 AI 隔离边界**。

---

## 二、数据风险分级

| 类型 | 风险级别 | 是否允许出云 |
|------|----------|-------------|
| 通用框架代码 | 低 | 可 |
| SQL 执行器 | 低 | 可 |
| Parser/Optimizer | 低 | 可 |
| UI 组件 | 低 | 可 |
| 企业 SOP | 高 | 禁止 |
| 批生产记录（EBR） | 极高 | 禁止 |
| 配方工艺参数 | 极高 | 禁止 |
| 偏差/CAPA 数据 | 高 | 禁止 |
| 电子签名链 | 极高 | 禁止 |
| 审计日志 | 极高 | 禁止 |
| 客户数据库 | 极高 | 禁止 |
| LIMS/MES 数据 | 极高 | 禁止 |
| 向量记忆库 | 极高 | 禁止 |
| Agent 长期记忆 | 极高 | 禁止 |

---

## 三、四层 AI 隔离体系

### Layer 0：Air-Gap Trust Zone（绝对可信区）

GMP 核心区。完全本地。绝不联网。

包括：

- SQLRustGo 主数据库
- 审计链
- EBR
- 电子签名
- 批记录
- SOP
- MES/LIMS
- OPC-UA
- HSM/KMS
- 向量数据库
- Agent 长期记忆

模型：**本地 LLM ONLY**（Qwen3、DeepSeek 本地版、Llama、Mistral）

**禁止云模型访问。禁止 Agent 直接读取原始数据。** 只能通过 Policy Engine、Data Gateway、DSL Query 访问。

---

### Layer 1：Sanitized AI Zone（脱敏 AI 区）

最关键的一层。允许云模型参与，但**永远不允许真实数据进入**。

进入云端前必须经过：

#### 1. 数据脱敏

原始：

```json
{
  "batch": "BATCH-2026-001",
  "material": "mRNA-LNP-X7",
  "temperature": 4.2
}
```

脱敏后：

```json
{
  "batch": "BATCH-XXXX",
  "material": "FORMULA-X",
  "temperature": "RANGE_LOW"
}
```

#### 2. Schema 保留，值删除

云模型只看：

```sql
TABLE batch_record (
  id,
  created_at,
  status,
  workflow_state
)
```

而不看真实内容。

#### 3. 自动 Token 替换

```text
客户名称 -> CUSTOMER_A
药品名称 -> PRODUCT_X
工艺参数 -> PARAM_SET_12
```

#### 4. Prompt Firewall

所有 Prompt 必须经过 Prompt Sanitizer：

```text
禁止：
- API Key
- 患者数据
- 批号
- 签名
- 原始日志
```

---

### Layer 2：Cloud Reasoning Zone（云推理区）

GPT-5.5、Claude、Gemini、DeepSeek API 真正工作的地方。

但它们只看到：

- 抽象 DSL
- 脱敏 Schema
- Mock 数据
- 合成数据
- 中间 IR

而不是真正的 GMP 数据。

---

### Layer 3：Execution & Validation Zone（执行验证区）

云模型永远不能：直接执行、写数据库、提交生产变更。

必须经过**本地 Deterministic Validator**：

- SQL AST 验证
- 权限验证
- 审计验证
- Formal Gate
- Rule Engine
- TLA+/Coq
- Policy Engine

**AI 只能"建议"，真正执行的是本地可信执行器。**

---

## 四、Agent Memory 必须结构化

很多 AI 系统死在让 Agent 拥有原始上下文。

错误方式：

```text
Agent Memory: 客户A的疫苗生产异常...
```

这已经泄露了。

正确方式：

```json
{
  "workflow_type": "DEVIATION",
  "risk_level": "HIGH",
  "requires_signature": true
}
```

---

## 五、双仓开发

GMP-OS 开发时，Claude/GPT 非常强，本地模型不够。推荐"双仓开发"：

### 仓库 A：Core Open Source

允许云模型访问。包括：

- SQL 引擎
- Parser
- Optimizer
- Workflow Engine
- UI
- Generic Agent Framework

这是 Hermes/OpenClaw 的工作区。

### 仓库 B：Private GMP Layer

完全私有。包括：

- 企业 SOP
- 企业插件
- 配方
- EBR 模板
- 客户规则
- 审计策略
- 向量知识库

**禁止出云。**

---

## 六、AI Data Diode（AI 单向网闸）

类似军工单向网闸：

```
可信区 → 脱敏区 → 云区
```

允许摘要、Schema、中间 IR 出去。**禁止原始数据返回。**

这个概念非常适合 GMP。

---

## 七、AI Governance Engine

GMP-OS 应内建 AI Governance Engine，这是未来制药 AI 系统的核心，也是 GMP-OS 的战略护城河。

| 模块 | 功能 |
|------|------|
| Prompt Firewall | Prompt 脱敏 |
| Data Classifier | 数据分级 |
| PII Detector | 敏感检测 |
| Policy Engine | 出云策略 |
| Model Router | 模型路由 |
| Audit Chain | AI 调用审计 |
| Tokenizer | 实体替换 |
| Synthetic Generator | 合成数据 |
| Output Validator | 输出验证 |
| AI Risk Scorer | AI 风险评分 |

绝大多数 AI Agent 系统根本没有"可信 AI 架构"，能通过 GMP/Part11/Annex11 审计的 AI 平台会极少。

---

## 八、AI Sandboxed Execution

未来真正成熟的 GMP-OS，不是"AI 调用数据库"，而是**AI 调用受限 DSL**。

AI 只能生成：

```yaml
action: deviation_summary
filters:
  severity: HIGH
  month: 2026-05
```

然后本地 DSL Engine：权限检查 → 数据裁剪 → 审计 → 签名 → 输出脱敏 → 才返回。

这是 AI Sandboxed Execution，也是未来企业 AI 的标准形态。

---

## 九、战略建议

不要把"Hermes/OpenClaw + GPT"理解成 AI Coding Tool，而应该理解成**可信 AI 操作系统（Trusted AI OS）**。

核心不是模型有多强，而是**如何控制模型接触什么**。这是 GMP 世界真正的护城河。

未来：数据库公司很多、Agent 框架很多、大模型很多，但能通过 GMP/Part11/Annex11 审计的 AI 平台会极少。

下一步真正应该做的是：**Trusted AI Governance Layer（可信 AI 治理层）**——这会成为 GMP-OS 的核心壁垒。
