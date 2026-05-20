# SQLRustGo GMP-OS 演进路线图 (v3.4.0 → v4.0.0)

> **版本**: v1.0
> **创建日期**: 2026-05-18
> **维护人**: hermes-agent
> **分支**: `develop/v3.3.0`
> **关联文档**: [ENHANCEMENT_ROADMAP.md](./ENHANCEMENT_ROADMAP.md), [GMP_OS_ARCHITECTURE.md](../../gmp-os/GMP_OS_ARCHITECTURE.md)
> **代号**: Elastic GMP Platform → AI-Native GMP Operating System

---

## 一、总体演进目标

### 1.1 系统重新定位

SQLRustGo 不再定位为传统数据库或 ERP，而是：

```
SQLRustGo GMP Platform = AI-Native GMP Operating System
```

核心架构：

```
Mobile App
    ↓
HermesAgent (AI Workflow)
    ↓
SQLRustGo Regulatory Core
    ↓
Audit / Graph / Vector / Workflow / Storage
```

**设计哲学**：GMP 现场本质不是"表单录入"，而是"事件驱动 + 人机协同 + 审计留痕"。

### 1.2 系统分层定义

| 层 | 作用 | 技术实现 |
|----|------|---------|
| Regulatory Kernel | 合规内核 | `crates/regulatory-core` |
| Data Platform | 数据平台 | SQLRustGo Core (SQL/Graph/Vector/WAL) |
| AI Agent Runtime | AI 工作流 | `crates/hermes-runtime` + OpenClaw |
| Mobile Workflow | 移动生产 | Flutter App + gRPC |
| Audit Chain | 审计 | `crates/gmp-integrity` (Merkle Chain) |
| Knowledge Graph | GMP 知识图谱 | `crates/gmp-knowledge` |

### 1.3 演进阶段总览

| 版本 | 代号 | 主题 | 核心交付 | 建议周期 |
|------|------|------|---------|---------|
| v3.4.0 | Regulatory Foundation | GMP 合规基础设施 | Regulatory Core + ALCOA+ + Merkle Chain | 3-5 个月 |
| v3.5.0 | AI Workflow GMP | AI 驱动 GMP 工作流 | HermesAgent 集成 + AI 偏差/CAPA | 3-4 个月 |
| v3.6.0 | GMP Knowledge Graph | 制药知识图谱 | 图谱节点 + 根因分析 + 风险传播 | 3-4 个月 |
| v3.7.0 | Industrial Edge | 连接真实工厂 | OPC UA + MQTT + Edge Gateway | 3-4 个月 |
| v3.8.0 | Compliance Intelligence | AI 合规引擎 | AI 审计 + 法规解析 + 检查模拟 | 3-4 个月 |
| v3.9.0 | Elastic GMP Platform | 平台化 | Plugin SDK + Multi-Tenant + API Gateway | 4-5 个月 |
| v4.0.0 | AI-Native GMP OS | GMP 操作系统 | Agent First + Event Sourcing + Digital Twin | 5-6 个月 |

### 1.4 成功的定义

- [ ] 21 CFR Part 11 / EU Annex 11 合规验证通过
- [ ] ALCOA+ 原则全链路覆盖
- [ ] Merkle 审计链防篡改验证通过
- [ ] AI 偏差调查准确率 ≥85%
- [ ] Mobile App 覆盖 6 大现场场景
- [ ] 知识图谱根因分析覆盖率 ≥80%
- [ ] OPC UA/MQTT 工业协议对接成功
- [ ] 72h 混合负载稳定性测试无崩溃

---

## 二、v3.4.0 — Regulatory Foundation

### 2.1 版本目标

```
建立 GMP 合规基础设施
```

从数据库 → GMP 可信平台的关键一步。

### 2.2 新增 Crate

| Crate | 路径 | 职责 |
|-------|------|------|
| `sqlrustgo-regulatory-core` | `crates/regulatory-core` | 统一合规内核 |
| `sqlrustgo-gmp-integrity` | `crates/gmp-integrity` | ALCOA+ 数据完整性引擎 |

### 2.3 功能列表

#### 2.3.1 Regulatory Core

**统一合规内核**，整合现有 `crates/gmp` 中的分散功能：

```rust
// crates/regulatory-core/src/lib.rs

pub mod audit;          // 审计日志 (从 gmp::audit 迁移)
pub mod signature;      // 电子签名 (从 gmp::signature 迁移)
pub mod timestamp;      // 可信时间戳
pub mod compliance;     // 合规检查
pub mod risk;           // 风险评估
pub mod evidence;       // 证据链 (从 gmp::evidence 迁移)
pub mod policy;         // 合规策略引擎
```

**核心 trait**：

```rust
pub trait RegulatoryService {
    fn audit_log(&mut self, event: AuditEvent) -> RegulatoryResult<AuditEntryId>;
    fn sign(&mut self, signer: &Signer, document: &Document) -> RegulatoryResult<Signature>;
    fn verify_evidence(&self, evidence: &Evidence) -> RegulatoryResult<bool>;
    fn check_compliance(&self, operation: &Operation) -> RegulatoryResult<ComplianceReport>;
}
```

#### 2.3.2 ALCOA+ Engine

**数据完整性引擎**，实现 ALCOA+ 原则：

| 原则 | 实现 | 状态 |
|------|------|------|
| **A**ttributable | 操作者身份绑定 + 电子签名 | ✓ (现有) |
| **L**egible | 不可变记录格式 | ✓ (现有) |
| **C**ontemporaneous | 可信时间戳 + NTP 同步 | TODO |
| **O**riginal | 原始数据保留 + 版本链 | ✓ (现有) |
| **A**ccurate | 校验和 + 防篡改 | TODO |
| Complete | 完整性校验 | TODO |
| Consistent | 一致性验证 | TODO |
| Enduring | 持久化保证 | TODO |
| Available | 可用性保证 | TODO |

**新增类型**：

```rust
// crates/gmp-integrity/src/lib.rs

pub struct AlcoaPlusValidator {
    pub timestamp_provider: Arc<dyn TrustedTimestamp>,
    pub checksum_engine: Arc<dyn ChecksumEngine>,
    pub signature_verifier: Arc<dyn SignatureVerifier>,
}

impl AlcoaPlusValidator {
    pub fn validate_attributable(&self, entry: &AuditEntry) -> AlcoaResult<()>;
    pub fn validate_contemporaneous(&self, entry: &AuditEntry) -> AlcoaResult<()>;
    pub fn validate_accurate(&self, entry: &AuditEntry) -> AlcoaResult<()>;
    pub fn validate_complete(&self, entries: &[AuditEntry]) -> AlcoaResult<()>;
    pub fn validate_all(&self, entries: &[AuditEntry]) -> AlcoaPlusReport;
}
```

#### 2.3.3 Electronic Signature 1.0

**实现方案**：用户名 + 密码 + OTP

```rust
pub struct ElectronicSignature {
    pub signer_id: String,
    pub signer_name: String,
    pub signature_meaning: SignatureMeaning,  // approval, review, verification
    pub timestamp: TrustedTimestamp,
    pub otp_code: String,
    pub public_key: Option<PublicKey>,
}

pub enum SignatureMeaning {
    Approval,       // 审批
    Review,         // 审核
    Verification,   // 验证
    Execution,      // 执行
    Witness,        // 见证
}
```

**适用场景**：

| 场景 | 签名类型 | 要求 |
|------|----------|------|
| 批记录 | Execution + Approval | 双签名 |
| SOP 审批 | Review + Approval | 双签名 |
| 偏差关闭 | Review + Approval | QA 签名 |
| CAPA 批准 | Review + Approval | QA + QA 主管 |

#### 2.3.4 Audit Chain 2.0 (Merkle Chain)

**升级现有 `AuditChain` → `MerkleAuditChain`**：

```rust
// 从 gmp::audit_chain 升级

pub struct MerkleAuditChain {
    pub entries: Vec<MerkleAuditEntry>,
    pub merkle_tree: MerkleTree,
    pub genesis_hash: [u8; 32],
    pub snapshot_interval: u64,     // 每 N 条生成快照
    pub snapshots: Vec<ChainSnapshot>,
}

pub struct MerkleAuditEntry {
    pub index: u64,
    pub timestamp: TrustedTimestamp,
    pub operation: String,
    pub operator: String,
    pub resource_type: ResourceType,
    pub resource_id: String,
    pub before_state: Option<Vec<u8>>,
    pub after_state: Option<Vec<u8>>,
    pub entry_hash: [u8; 32],
    pub prev_hash: [u8; 32],
    pub merkle_proof: Vec<[u8; 32]>,
}

pub struct ChainSnapshot {
    pub snapshot_id: u64,
    pub merkle_root: [u8; 32],
    pub entry_count: u64,
    pub timestamp: TrustedTimestamp,
    pub prev_snapshot_hash: [u8; 32],
}

impl MerkleAuditChain {
    pub fn append(&mut self, entry: AuditEvent) -> Result<MerkleAuditEntry>;
    pub fn verify(&self) -> Result<bool>;
    pub fn verify_entry(&self, index: u64) -> Result<bool>;
    pub fn generate_proof(&self, index: u64) -> Result<MerkleProof>;
    pub fn verify_proof(&self, proof: &MerkleProof) -> Result<bool>;
    pub fn snapshot(&mut self) -> Result<ChainSnapshot>;
    pub fn detect_tamper(&self) -> Result<Vec<TamperDetection>>;
}
```

#### 2.3.5 GMP Workflow 1.0

**事件驱动工作流引擎**：

```rust
pub struct GmpWorkflowEngine {
    pub event_bus: Arc<EventBus>,
    pub rule_engine: Arc<RuleEngine>,
    pub state_machine: Arc<StateMachine>,
}

// 事件驱动示例
pub enum GmpEvent {
    TemperatureExceeded { device_id: String, value: f64, threshold: f64 },
    CalibrationDue { device_id: String, due_date: DateTime },
    DeviationOpened { deviation_id: String, severity: Severity },
    BatchStarted { batch_id: String, product: String },
    SignatureRequested { document_id: String, signer: String },
}

// 工作流规则
pub struct WorkflowRule {
    pub trigger: GmpEvent,
    pub conditions: Vec<Condition>,
    pub actions: Vec<WorkflowAction>,
}

// 示例：温度超限 → 自动偏差
// TemperatureExceeded → CreateDeviation → NotifyQA → AssignCAPA
```

#### 2.3.6 Mobile First

**优先级**：Mobile App > Web ERP

**原因**：GMP 现场操作（巡检、校准、清洁、生产、点检）均在移动端。

**App 功能**：

| 功能 | 描述 | 优先级 |
|------|------|--------|
| SOP 扫码 | 扫码查看 SOP | P0 |
| 电子签名 | 手机确认签名 | P0 |
| 偏差上报 | 拍照上传 + AI 辅助 | P0 |
| AI 助手 | HermesAgent 语音交互 | P1 |
| 批记录 | 移动录入 | P1 |
| 环境监测 | 实时报警推送 | P1 |

**技术选型**：Flutter + gRPC

### 2.4 门禁条件

#### Alpha Gate

- [ ] `crates/regulatory-core` 编译通过
- [ ] `crates/gmp-integrity` 编译通过
- [ ] ALCOA+ 9 项原则全部可验证
- [ ] MerkleAuditChain append + verify 通过
- [ ] 电子签名 OTP 流程可用

#### Beta Gate

- [ ] 21 CFR Part 11 核心条款验证通过
- [ ] Merkle Chain 防篡改测试通过 (1000 次篡改检测)
- [ ] GMP Workflow 3 条事件链可运行
- [ ] Mobile App 原型 3 个核心功能可用
- [ ] 无 P0/P1 Bug

---

## 三、v3.5.0 — AI Workflow GMP

### 3.1 版本目标

```
AI 驱动 GMP 工作流
```

### 3.2 新增 Crate

| Crate | 路径 | 职责 |
|-------|------|------|
| `sqlrustgo-hermes-runtime` | `crates/hermes-runtime` | HermesAgent AI 运行时 |

### 3.3 AI Agent 架构

```
Mobile App
   ↓  gRPC
HermesAgent
   ↓  Workflow Engine
SQLRustGo Regulatory Core
   ↓  Graph / Vector / Audit
Data Layer
```

### 3.4 功能列表

#### 3.4.1 AI 偏差调查

**用户输入**：冻干机温度异常

**AI 自动执行**：

```
1. 查设备 → Graph: Device → Calibration → Status
2. 查校准 → Graph: Device → LastCalibrationDate
3. 查批次 → Graph: Device → Batch → Parameters
4. 查 SOP → Vector: SOP Embedding → Similarity Search
5. 查历史偏差 → Graph: Device → Deviation → History
6. 查法规 → Vector: Regulation Embedding → Relevant Clauses
```

**输出**：Deviation Draft (偏差报告草稿)

```rust
pub struct DeviationDraft {
    pub title: String,
    pub description: String,
    pub affected_batches: Vec<String>,
    pub device_history: DeviceHistory,
    pub relevant_sops: Vec<SopReference>,
    pub historical_deviations: Vec<DeviationReference>,
    pub regulatory_impact: Vec<RegulatoryClause>,
    pub suggested_severity: Severity,
    pub suggested_capa: Option<CapaSuggestion>,
}
```

#### 3.4.2 AI CAPA

**自动生成**：

```rust
pub struct CapaSuggestion {
    pub root_cause: RootCauseAnalysis,
    pub corrective_actions: Vec<ActionItem>,
    pub preventive_actions: Vec<ActionItem>,
    pub effectiveness_check: EffectivenessCheck,
    pub timeline: Timeline,
}

pub struct RootCauseAnalysis {
    pub methodology: RootCauseMethod,  // 5Why, Fishbone, FMEA
    pub causes: Vec<CauseChain>,
    pub evidence: Vec<EvidenceReference>,
    pub confidence: f64,
}
```

#### 3.4.3 AI Batch Review

**AI 自动审核批记录**：

```rust
pub struct BatchReviewDraft {
    pub batch_id: String,
    pub parameter_review: ParameterReview,
    pub deviation_review: DeviationReview,
    pub signature_review: SignatureReview,
    pub environmental_review: EnvironmentalReview,
    pub overall_assessment: OverallAssessment,
    pub recommendation: ReviewRecommendation,
}
```

#### 3.4.4 AI SOP Assistant

**现场语音交互**：

```
用户: "这个设备怎么清洁？"
AI:
  1. 识别设备 (扫码/语音)
  2. 检索 SOP (Vector Search)
  3. 检查培训记录 (Graph Query)
  4. 评估风险 (Risk Engine)
  5. 语音回答关键步骤
```

### 3.5 HermesAgent Runtime

```rust
// crates/hermes-runtime/src/lib.rs

pub struct HermesRuntime {
    pub llm_client: Arc<dyn LlmClient>,
    pub tool_registry: Arc<ToolRegistry>,
    pub workflow_engine: Arc<GmpWorkflowEngine>,
    pub knowledge_base: Arc<dyn KnowledgeBase>,
    pub audit_logger: Arc<dyn AuditLogger>,
}

pub trait GmpAgent {
    async fn investigate_deviation(&self, context: &DeviationContext) -> AgentResult<DeviationDraft>;
    async fn generate_capa(&self, deviation: &Deviation) -> AgentResult<CapaSuggestion>;
    async fn review_batch(&self, batch_id: &str) -> AgentResult<BatchReviewDraft>;
    async fn assist_sop(&self, query: &str, device_id: &str) -> AgentResult<SopAssistance>;
}
```

### 3.6 门禁条件

- [ ] HermesAgent Runtime 可运行
- [ ] AI 偏差调查准确率 ≥80%
- [ ] AI CAPA 生成可用
- [ ] AI Batch Review 可运行
- [ ] SOP Assistant 语音交互可用

---

## 四、v3.6.0 — GMP Knowledge Graph

### 4.1 版本目标

```
建立制药知识图谱
```

### 4.2 新增 Crate

| Crate | 路径 | 职责 |
|-------|------|------|
| `sqlrustgo-gmp-knowledge` | `crates/gmp-knowledge` | GMP 知识图谱 |

### 4.3 图谱节点

| 节点 | Label | 属性 | 关系 |
|------|-------|------|------|
| Batch | `Batch` | batch_id, product, date, status | → Device, → Operator, → Deviation |
| Device | `Device` | device_id, name, type, status | → Calibration, → SOP, → Deviation |
| SOP | `SOP` | sop_id, title, version, effective_date | → Operator (培训), → Device |
| Operator | `Operator` | operator_id, name, role, qualifications | → Batch, → Signature |
| CAPA | `CAPA` | capa_id, type, status, due_date | → Deviation, → Operator |
| Deviation | `Deviation` | deviation_id, severity, status | → Batch, → Device, → Regulation |
| Regulation | `Regulation` | reg_id, authority, clause, text | → Deviation, → SOP |

### 4.4 图谱能力

#### 4.4.1 根因分析

```cypher
// 哪些设备导致最多偏差
MATCH (d:Device)-[:HAS_DEVIATION]->(dev:Deviation)
RETURN d.device_id, d.name, count(dev) as deviation_count
ORDER BY deviation_count DESC
LIMIT 10
```

#### 4.4.2 风险传播

```cypher
// 设备污染 → 哪些批次受影响
MATCH (d:Device {device_id: $device_id})-[:USED_IN]->(b:Batch)
WHERE b.status = 'in_progress'
RETURN b.batch_id, b.product, b.start_date
```

#### 4.4.3 法规关联

```cypher
// 偏差自动关联法规
MATCH (dev:Deviation {deviation_id: $dev_id})-[:RELATES_TO]->(r:Regulation)
RETURN r.authority, r.clause, r.text
```

### 4.5 与现有 Graph 模块集成

```
crates/graph (现有)          crates/gmp-knowledge (新增)
├── cypher/                  ├── schema/         (GMP 图谱 Schema)
│   ├── parser               ├── loaders/        (数据加载器)
│   └── executor             ├── analytics/      (图分析算法)
├── model/                   ├── regulations/    (法规库)
│   ├── node                 └── queries/        (预定义查询)
│   └── edge
└── store/
    ├── graph_store
    ├── adjacency_index
    └── label_index
```

### 4.6 门禁条件

- [ ] GMP 知识图谱 7 类节点可创建
- [ ] 根因分析 Cypher 查询可执行
- [ ] 风险传播路径可追踪
- [ ] 法规关联准确率 ≥85%

---

## 五、v3.7.0 — Industrial Edge

### 5.1 版本目标

```
连接真实工厂
```

### 5.2 新增 Crate

| Crate | 路径 | 职责 |
|-------|------|------|
| `sqlrustgo-gmp-iot` | `crates/gmp-iot` | 工业物联网连接 |

### 5.3 功能列表

#### 5.3.1 OPC UA 支持

```rust
pub struct OpcUaConnector {
    pub endpoint: String,
    pub security_mode: SecurityMode,
    pub nodes: Vec<OpcUaNode>,
}

impl OpcUaConnector {
    pub async fn connect(&mut self) -> IotResult<()>;
    pub async fn read_node(&self, node_id: &str) -> IotResult<DataValue>;
    pub async fn subscribe(&mut self, node_id: &str, callback: OnChange) -> IotResult<()>;
}
```

**支持设备**：PLC、SCADA、DCS

#### 5.3.2 MQTT 采集

```rust
pub struct MqttCollector {
    pub broker: String,
    pub topics: Vec<MqttTopic>,
    pub buffer: DataBuffer,
}

// 采集数据类型
pub enum EnvironmentalData {
    Temperature { value: f64, unit: String, location: String },
    Humidity { value: f64, unit: String, location: String },
    PressureDiff { value: f64, unit: String, location: String },
    ParticleCount { value: u64, size_um: f64, location: String },
}
```

#### 5.3.3 Edge Gateway

```rust
pub struct EdgeGateway {
    pub local_store: Arc<dyn StorageEngine>,
    pub sync_manager: Arc<SyncManager>,
    pub offline_buffer: OfflineBuffer,
}

impl EdgeGateway {
    pub async fn go_offline(&mut self) -> IotResult<()>;
    pub async fn go_online(&mut self) -> IotResult<()>;
    pub async fn sync_pending(&mut self) -> IotResult<SyncReport>;
    pub async fn resolve_conflicts(&mut self, conflicts: Vec<Conflict>) -> IotResult<()>;
}
```

**核心能力**：离线生产 + 恢复同步

#### 5.3.4 实时事件流

```rust
pub struct EventBus {
    pub subscribers: HashMap<String, Vec<Box<dyn EventHandler>>>,
    pub buffer_size: usize,
}

// 事件链示例
// 设备报警 → 自动偏差
// TemperatureExceeded → CreateDeviation → NotifyQA → AssignCAPA
```

### 5.4 门禁条件

- [ ] OPC UA 连接器可连接模拟 PLC
- [ ] MQTT 采集温湿度数据成功
- [ ] Edge Gateway 离线/在线切换成功
- [ ] 事件总线 3 条事件链可运行

---

## 六、v3.8.0 — Compliance Intelligence

### 6.1 版本目标

```
AI 合规引擎
```

### 6.2 新增 Crate

| Crate | 路径 | 职责 |
|-------|------|------|
| `sqlrustgo-regulatory-ai` | `crates/regulatory-ai` | AI 合规引擎 |

### 6.3 功能列表

#### 6.3.1 AI 审计

**自动发现异常**：

| 异常类型 | 检测方法 | 置信度 |
|----------|----------|--------|
| 数据回填 | 时间戳分析 + 统计异常 | ≥90% |
| 数据篡改 | Merkle Proof 验证 | 100% |
| 异常签名 | 签名模式分析 | ≥85% |
| 越权操作 | 权限图遍历 | ≥90% |

```rust
pub struct ComplianceAuditor {
    pub anomaly_detectors: Vec<Box<dyn AnomalyDetector>>,
    pub audit_chain: Arc<MerkleAuditChain>,
    pub knowledge_graph: Arc<GmpKnowledgeGraph>,
}

impl ComplianceAuditor {
    pub async fn audit(&self, scope: &AuditScope) -> ComplianceAuditReport;
    pub async fn detect_backdating(&self, entries: &[MerkleAuditEntry]) -> Vec<Anomaly>;
    pub async fn detect_tampering(&self, entries: &[MerkleAuditEntry]) -> Vec<Anomaly>;
    pub async fn detect_unauthorized(&self, entries: &[MerkleAuditEntry]) -> Vec<Anomaly>;
}
```

#### 6.3.2 AI 法规解析

```rust
pub struct RegulationParser {
    pub llm_client: Arc<dyn LlmClient>,
    pub regulation_db: Arc<dyn RegulationDatabase>,
}

impl RegulationParser {
    pub async fn parse_new_regulation(&self, text: &str) -> ParsedRegulation;
    pub async fn assess_impact(&self, regulation: &ParsedRegulation) -> ImpactAssessment;
    pub async fn identify_affected_sops(&self, regulation: &ParsedRegulation) -> Vec<SopImpact>;
    pub async fn identify_training_needs(&self, regulation: &ParsedRegulation) -> Vec<TrainingNeed>;
}
```

#### 6.3.3 AI Inspection Simulator

**模拟 FDA/EMA 检查**：

```rust
pub struct InspectionSimulator {
    pub knowledge_graph: Arc<GmpKnowledgeGraph>,
    pub audit_chain: Arc<MerkleAuditChain>,
    pub compliance_engine: Arc<ComplianceAuditor>,
}

impl InspectionSimulator {
    pub async fn simulate_fda_inspection(&self, scope: &InspectionScope) -> InspectionReport;
    pub async fn generate_observations(&self) -> Vec<FdaObservation>;
    pub async fn generate_response_draft(&self, observations: &[FdaObservation]) -> Vec<ResponseDraft>;
}
```

### 6.4 门禁条件

- [ ] AI 审计 4 类异常检测可用
- [ ] 法规解析准确率 ≥80%
- [ ] FDA 检查模拟可生成观察项
- [ ] 483 Response Draft 可生成

---

## 七、v3.9.0 — Elastic GMP Platform

### 7.1 版本目标

```
真正的平台化
```

### 7.2 新增 Crate

| Crate | 路径 | 职责 |
|-------|------|------|
| `sqlrustgo-plugin-sdk` | `crates/plugin-sdk` | 插件开发 SDK |
| `sqlrustgo-api-gateway` | `crates/api-gateway` | API 网关 |

### 7.3 功能列表

#### 7.3.1 Plugin System

```rust
pub trait GmpPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn on_load(&mut self, context: &PluginContext) -> PluginResult<()>;
    fn on_event(&mut self, event: &GmpEvent) -> PluginResult<()>;
    fn on_unload(&mut self) -> PluginResult<()>;
}

pub struct PluginContext {
    pub event_bus: Arc<EventBus>,
    pub audit_chain: Arc<MerkleAuditChain>,
    pub knowledge_graph: Arc<GmpKnowledgeGraph>,
    pub config: PluginConfig,
}
```

**插件类型**：
- AI 插件 (自定义 Agent)
- Workflow 插件 (自定义工作流)
- 审计插件 (自定义审计规则)

#### 7.3.2 Multi-Tenant

```rust
pub struct TenantManager {
    pub tenants: HashMap<TenantId, Tenant>,
    pub resource_quotas: HashMap<TenantId, ResourceQuota>,
}

pub struct Tenant {
    pub id: TenantId,
    pub name: String,
    pub factory_id: String,
    pub schema: String,
    pub isolation_level: IsolationLevel,
}
```

**支持**：多工厂、多租户、SaaS

#### 7.3.3 API Gateway

```
统一入口：
├── Mobile App    → gRPC
├── AI Agent      → gRPC
├── IoT Device    → MQTT
├── ERP System    → REST
└── Web Dashboard → WebSocket
```

### 7.4 门禁条件

- [ ] Plugin SDK 可加载自定义插件
- [ ] Multi-Tenant 隔离验证通过
- [ ] API Gateway 4 种协议可接入
- [ ] 无 P0/P1 Bug

---

## 八、v4.0.0 — AI-Native GMP OS

### 8.1 版本目标

```
GMP Operating System
```

不是 ERP。是生产现场的 AI 操作系统。

### 8.2 核心思想

```
生产现场 = 人 + AI + 设备 共同工作
```

### 8.3 v4.0 核心特性

#### 8.3.1 AI Agent First

**交互范式变革**：

```
传统: 用户点菜单 → 填表单 → 提交
v4.0: 用户对 AI 说 → AI 执行 → AI 确认
```

**示例**：

```
用户: "启动今天冻干生产"

AI 自动执行:
1. 检查设备状态 → Graph Query
2. 检查校准有效期 → Graph Query
3. 检查环境参数 → IoT Stream
4. 检查操作员培训 → Graph Query
5. 创建批次记录 → SQL Insert
6. 下发生产任务 → Workflow Trigger
7. 开始数据采集 → IoT Subscribe
```

```rust
pub struct GmpAgentOrchestrator {
    pub agents: HashMap<AgentRole, Box<dyn GmpAgent>>,
    pub workflow_engine: Arc<GmpWorkflowEngine>,
    pub audit_chain: Arc<MerkleAuditChain>,
    pub knowledge_graph: Arc<GmpKnowledgeGraph>,
    pub iot_bus: Arc<EventBus>,
}

impl GmpAgentOrchestrator {
    pub async fn execute_command(&self, command: &str, context: &UserContext) -> AgentResult<ExecutionReport>;
    pub async fn autonomous_review(&self, batch_id: &str) -> AgentResult<AutonomousReview>;
    pub async fn predict_risk(&self, scope: &RiskScope) -> AgentResult<RiskPrediction>;
}
```

#### 8.3.2 Event Sourcing

**核心架构升级**：一切皆事件

```rust
pub enum GmpDomainEvent {
    BatchStarted { batch_id: String, product: String, operator: String },
    BatchCompleted { batch_id: String, yield_amount: f64 },
    TemperatureExceeded { device_id: String, value: f64, threshold: f64 },
    DeviationOpened { deviation_id: String, severity: Severity },
    DeviationClosed { deviation_id: String, resolution: String },
    CapaCreated { capa_id: String, deviation_id: String },
    SignatureApplied { document_id: String, signer: String, meaning: SignatureMeaning },
    CalibrationPerformed { device_id: String, result: CalibrationResult },
    SopApproved { sop_id: String, version: String },
    EnvironmentalAlert { location: String, parameter: String, value: f64 },
}

pub struct EventStore {
    pub storage: Arc<dyn StorageEngine>,
    pub audit_chain: Arc<MerkleAuditChain>,
    pub event_bus: Arc<EventBus>,
}

impl EventStore {
    pub async fn append(&mut self, event: GmpDomainEvent) -> EventStoreResult<EventId>;
    pub async fn replay(&self, from: EventId, to: EventId) -> EventStoreResult<Vec<GmpDomainEvent>>;
    pub async fn project(&self, aggregate_id: &str) -> EventStoreResult<AggregateState>;
}
```

#### 8.3.3 Digital Twin

**数字工厂**：实时映射物理工厂

```rust
pub struct DigitalTwin {
    pub devices: HashMap<String, DeviceTwin>,
    pub environments: HashMap<String, EnvironmentTwin>,
    pub batches: HashMap<String, BatchTwin>,
    pub event_bus: Arc<EventBus>,
}

pub struct DeviceTwin {
    pub device_id: String,
    pub real_time_state: DeviceState,
    pub calibration_status: CalibrationStatus,
    pub maintenance_schedule: MaintenanceSchedule,
    pub historical_deviations: Vec<DeviationSummary>,
}

impl DigitalTwin {
    pub async fn sync_from_iot(&mut self, data: &EnvironmentalData) -> TwinResult<()>;
    pub async fn predict_failure(&self, device_id: &str) -> TwinResult<FailurePrediction>;
    pub async fn simulate_scenario(&self, scenario: &Scenario) -> TwinResult<SimulationResult>;
}
```

#### 8.3.4 Autonomous QA

**QA 角色变革**：从手工审核 → 监督 AI

```rust
pub struct AutonomousQA {
    pub review_agent: Arc<dyn ReviewAgent>,
    pub compliance_agent: Arc<dyn ComplianceAgent>,
    pub risk_agent: Arc<dyn RiskAgent>,
    pub human_override: Arc<HumanOverrideChannel>,
}

impl AutonomousQA {
    pub async fn auto_review_batch(&self, batch_id: &str) -> QaResult<AutonomousReview>;
    pub async fn auto_check_compliance(&self, operation: &Operation) -> QaResult<ComplianceVerdict>;
    pub async fn auto_risk_assess(&self, change: &ChangeRequest) -> QaResult<RiskAssessment>;
    pub async fn escalate_to_human(&self, issue: &QaIssue) -> QaResult<()>;
}
```

### 8.4 门禁条件

- [ ] AI Agent First 5 个生产场景可运行
- [ ] Event Sourcing 事件回放正确性验证
- [ ] Digital Twin 实时同步延迟 ≤1s
- [ ] Autonomous QA 准确率 ≥90%
- [ ] 21 CFR Part 11 全条款合规
- [ ] 72h 生产模拟稳定性测试通过

---

## 九、技术路线

### 9.1 技术选型

| 层 | 技术 | 原因 |
|----|------|------|
| Core | Rust | 性能 + 安全 + 合规 |
| AI Runtime | OpenClaw / Hermes | AI Agent 框架 |
| Mobile | Flutter | 跨平台 + 离线支持 |
| API | gRPC | 高性能 + 类型安全 |
| Event Bus | NATS | 轻量 + 云原生 |
| TimeSeries | SQLRustGo TSDB | 自研 + 统一栈 |
| Workflow | 自研 Event Workflow | GMP 专用 |
| Audit | Merkle Chain | 防篡改 + 可验证 |

### 9.2 不建议的技术

| 技术 | 原因 |
|------|------|
| 重型 Java ERP | 太慢、太重、太难升级、不适合 AI |
| 传统 MIS | 不适合事件驱动 GMP |
| 通用 MES | 过度复杂、定制困难 |

### 9.3 与 v3.3.0 ENHANCEMENT_ROADMAP 的关系

| v3.3.0 计划项 | GMP-OS 版本 | 说明 |
|--------------|------------|------|
| P0: 核心稳定性 | v3.3.0 先行 | 数据库内核必须稳定 |
| P1: 性能竞争力 | v3.4.0 并行 | Regulatory Core 需要高性能 |
| P2: 运维可观测 | v3.5.0+ | AI 工作流需要可观测 |
| P3: 生态体验 | v3.9.0 | 平台化需要生态 |

---

## 十、风险评估

### 10.1 已识别风险

| 风险 | 概率 | 影响 | 风险值 | 缓解措施 |
|------|------|------|--------|----------|
| 21 CFR Part 11 合规不通过 | MEDIUM | CRITICAL | CRITICAL | 提前引入合规顾问 |
| AI 偏差调查准确率不达标 | MEDIUM | HIGH | HIGH | 人机协同 + 逐步自动化 |
| OPC UA 协议兼容性 | MEDIUM | MEDIUM | MEDIUM | 先支持主流 PLC |
| Mobile App 离线同步冲突 | HIGH | HIGH | CRITICAL | CRDT + 冲突解决策略 |
| Event Sourcing 性能瓶颈 | LOW | HIGH | MEDIUM | 快照 + 分区 |
| Digital Twin 实时性 | MEDIUM | MEDIUM | MEDIUM | 边缘计算 + 本地缓存 |

### 10.2 合规风险

| 法规 | 风险点 | 缓解措施 |
|------|--------|----------|
| 21 CFR Part 11 | 电子签名法律效力 | HSM + PKI + 审计链 |
| EU Annex 11 | 计算机系统验证 | CSV 协议 + 验证报告 |
| PIC/S | 药品检查合作 | 知识图谱法规关联 |
| GAMP 5 | 自动化系统验证 | 风险分级验证策略 |

---

## 十一、里程碑

| 里程碑 | 目标日期 | 说明 |
|--------|----------|------|
| M1: v3.4.0 Alpha | 2026-08-15 | Regulatory Core + ALCOA+ |
| M2: v3.4.0 GA | 2026-10-15 | Merkle Chain + Mobile 原型 |
| M3: v3.5.0 Alpha | 2026-12-15 | HermesAgent 集成 |
| M4: v3.5.0 GA | 2027-02-15 | AI 偏差/CAPA/Batch Review |
| M5: v3.6.0 Alpha | 2027-03-15 | GMP 知识图谱 |
| M6: v3.7.0 Alpha | 2027-06-15 | Industrial Edge |
| M7: v3.8.0 Alpha | 2027-09-15 | Compliance Intelligence |
| M8: v3.9.0 Alpha | 2027-12-15 | Elastic Platform |
| M9: v4.0.0 Alpha | 2028-03-15 | AI-Native GMP OS |
| M10: v4.0.0 GA | 2028-06-15 | 生产就绪 |

---

## 十二、附录

### A.1 相关文档

| 文档 | 路径 | 说明 |
|------|------|------|
| GMP-OS 架构设计 | `docs/gmp-os/GMP_OS_ARCHITECTURE.md` | 详细架构设计 |
| v3.3.0 增强路线图 | `docs/releases/v3.3.0/ENHANCEMENT_ROADMAP.md` | 数据库增强计划 |
| Code Wiki | `docs/CODE_WIKI.md` | 项目架构文档 |
| DEV_PLAN | `docs/releases/v3.3.0/DEV_PLAN.md` | v3.3.0 开发计划 |

### A.2 现有 GMP 模块映射

| 现有模块 | GMP-OS 目标 | 迁移策略 |
|----------|------------|---------|
| `gmp::audit` | `regulatory-core::audit` | 直接迁移 |
| `gmp::audit_chain` | `gmp-integrity::merkle_chain` | 升级为 Merkle |
| `gmp::signature` | `regulatory-core::signature` | 迁移 + OTP |
| `gmp::evidence` | `regulatory-core::evidence` | 迁移 + ALCOA+ |
| `gmp::correction` | `regulatory-core::correction` | 迁移 |
| `gmp::document` | `regulatory-core::document` | 迁移 |
| `gmp::workflow` | `regulatory-core::workflow` | 升级为事件驱动 |
| `gmp::provenance` | `gmp-knowledge::provenance` | 迁移至知识图谱 |
| `gmp::hsm` | `regulatory-core::hsm` | 迁移 |
| `gmp::mobile` | Mobile App (Flutter) | 独立项目 |
| `gmp::sop` | `gmp-knowledge::sop` | 迁移至知识图谱 |
| `gmp::calibration` | `gmp-iot::calibration` | 迁移至 IoT 模块 |

### A.3 变更日志

| 日期 | 版本 | 变更 | 作者 |
|------|------|------|------|
| 2026-05-18 | v1.0 | 初始版本 | hermes-agent |

---

*本文档定义了 SQLRustGo 从数据库到 AI-Native GMP Operating System 的完整演进路线。*
