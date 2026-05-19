# SQLRustGo GMP-OS 架构设计

> **版本**: v1.0
> **创建日期**: 2026-05-18
> **维护人**: hermes-agent
> **关联文档**: [GMP_OS_ROADMAP.md](./GMP_OS_ROADMAP.md)
> **架构代号**: Elastic GMP Platform

---

## 一、架构定位

### 1.1 核心定义

```
SQLRustGo GMP-OS ≠ ERP
SQLRustGo GMP-OS ≠ MES
SQLRustGo GMP-OS = AI-Native GMP Operating System
```

**设计哲学**：

- GMP 现场是**事件驱动**的，不是表单驱动的
- GMP 操作是**人机协同**的，不是纯人工的
- GMP 合规是**审计留痕**的，不是事后补录的

### 1.2 与传统系统对比

| 维度 | 传统 ERP/MES | SQLRustGo GMP-OS |
|------|-------------|------------------|
| 交互方式 | 菜单 + 表单 | AI Agent + 语音 |
| 数据模型 | 关系表 | Event Sourcing + Graph |
| 合规方式 | 事后审计 | 实时审计链 |
| 部署方式 | 中心化 | 边缘 + 云 |
| 扩展方式 | 定制开发 | Plugin SDK |
| 移动支持 | Web 响应式 | Mobile First |

---

## 二、系统架构

### 2.1 总体架构图

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          Presentation Layer                             │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐  ┌─────────┐  │
│  │  Mobile App   │  │  AI Assistant │  │  Web Dashboard│  │ ERP API │  │
│  │  (Flutter)    │  │  (Voice/Chat) │  │  (React)      │  │ (REST)  │  │
│  └───────┬───────┘  └───────┬───────┘  └───────┬───────┘  └────┬────┘  │
└──────────┼──────────────────┼──────────────────┼───────────────┼────────┘
           │                  │                  │               │
┌──────────┼──────────────────┼──────────────────┼───────────────┼────────┐
│          │         API Gateway (gRPC/REST/MQTT) │               │        │
│  ┌───────┴──────────────────┴──────────────────┴───────────────┴─────┐  │
│  │                    crates/api-gateway                              │  │
│  └──────────────────────────────┬────────────────────────────────────┘  │
│                                 │                                       │
│  Integration Layer              │                                       │
├─────────────────────────────────┼───────────────────────────────────────┤
│                                 │                                       │
│  ┌───────────────┐  ┌──────────┴───────┐  ┌───────────────┐            │
│  │ HermesAgent   │  │ Regulatory       │  │ GMP IoT       │            │
│  │ Runtime       │  │ Kernel           │  │ Gateway       │            │
│  │ (AI Workflow) │  │ (Compliance)     │  │ (Edge)        │            │
│  └───────┬───────┘  └───────┬──────────┘  └───────┬───────┘            │
│          │                  │                      │                    │
│  ┌───────┴──────────────────┴──────────────────────┴───────┐            │
│  │              Domain Services Layer                       │            │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐   │            │
│  │  │ Batch    │ │ Deviation│ │ CAPA     │ │ SOP      │   │            │
│  │  │ Service  │ │ Service  │ │ Service  │ │ Service  │   │            │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘   │            │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐   │            │
│  │  │ Calibr.  │ │ Document │ │ Signature│ │ Audit    │   │            │
│  │  │ Service  │ │ Service  │ │ Service  │ │ Service  │   │            │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘   │            │
│  └──────────────────────────┬──────────────────────────────┘            │
│                             │                                           │
│  Data & Intelligence Layer  │                                           │
├─────────────────────────────┼───────────────────────────────────────────┤
│                             │                                           │
│  ┌───────────────┐  ┌──────┴──────┐  ┌───────────────┐                 │
│  │ GMP Knowledge │  │ GMP         │  │ Regulatory    │                 │
│  │ Graph         │  │ Integrity   │  │ AI            │                 │
│  │ (Neo4j-style) │  │ (ALCOA+)    │  │ (Compliance)  │                 │
│  └───────┬───────┘  └───────┬─────┘  └───────┬───────┘                 │
│          │                  │                 │                          │
│  ┌───────┴──────────────────┴─────────────────┴───────┐                 │
│  │              SQLRustGo Core Engine                  │                 │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────┐│                 │
│  │  │ SQL      │ │ Graph    │ │ Vector   │ │ WAL    ││                 │
│  │  │ Engine   │ │ Engine   │ │ Engine   │ │ Engine ││                 │
│  │  └──────────┘ └──────────┘ └──────────┘ └────────┘│                 │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────┐│                 │
│  │  │ Storage  │ │ Transact │ │ Distrib  │ │ Column ││                 │
│  │  │ Engine   │ │ Manager  │ │ Layer    │ │ Store  ││                 │
│  │  └──────────┘ └──────────┘ └──────────┘ └────────┘│                 │
│  └────────────────────────────────────────────────────┘                 │
│                                                                         │
│  Infrastructure Layer                                                   │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐     │
│  │ Event    │ │ Merkle   │ │ HSM      │ │ TimeSync │ │ Cluster  │     │
│  │ Store    │ │ Audit    │ │ (PKI)    │ │ (NTP/PTP)│ │ (Raft)   │     │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘     │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.2 数据流架构

```
┌──────────┐     ┌──────────┐     ┌──────────────┐     ┌──────────────┐
│  IoT     │────→│  Event   │────→│  Workflow    │────→│  Regulatory  │
│  Sensor  │     │  Bus     │     │  Engine      │     │  Kernel      │
└──────────┘     └────┬─────┘     └──────┬───────┘     └──────┬───────┘
                      │                  │                     │
                      ▼                  ▼                     ▼
               ┌──────────┐     ┌──────────────┐     ┌──────────────┐
               │  Event   │     │  AI Agent    │     │  Merkle      │
               │  Store   │     │  (Hermes)    │     │  Audit Chain │
               └──────────┘     └──────┬───────┘     └──────────────┘
                                       │
                              ┌────────┼────────┐
                              ▼        ▼        ▼
                        ┌────────┐┌────────┐┌────────┐
                        │Knowledge││Vector  ││SQL     │
                        │Graph   ││Index   ││Engine  │
                        └────────┘└────────┘└────────┘
```

---

## 三、核心 Crate 设计

### 3.1 crates/regulatory-core

**职责**：统一合规内核，整合审计、签名、时间、合规、风险、证据

```rust
// crates/regulatory-core/src/lib.rs

pub mod audit;
pub mod signature;
pub mod timestamp;
pub mod compliance;
pub mod risk;
pub mod evidence;
pub mod policy;
pub mod correction;
pub mod document;

pub trait RegulatoryService: Send + Sync {
    fn audit_log(&mut self, event: AuditEvent) -> RegulatoryResult<AuditEntryId>;
    fn sign(&mut self, signer: &Signer, document: &Document) -> RegulatoryResult<Signature>;
    fn verify_evidence(&self, evidence: &Evidence) -> RegulatoryResult<bool>;
    fn check_compliance(&self, operation: &Operation) -> RegulatoryResult<ComplianceReport>;
    fn assess_risk(&self, change: &ChangeRequest) -> RegulatoryResult<RiskAssessment>;
}

pub struct RegulatoryKernel {
    pub audit_service: Arc<dyn AuditService>,
    pub signature_service: Arc<dyn SignatureService>,
    pub timestamp_service: Arc<dyn TrustedTimestamp>,
    pub compliance_engine: Arc<dyn ComplianceEngine>,
    pub risk_engine: Arc<dyn RiskEngine>,
    pub evidence_store: Arc<dyn EvidenceStore>,
}
```

**依赖**：
- `sqlrustgo-types`
- `sqlrustgo-storage`
- `sqlrustgo-transaction`
- `sqlrustgo-gmp-integrity`

### 3.2 crates/gmp-integrity

**职责**：ALCOA+ 数据完整性引擎 + Merkle 审计链

```rust
// crates/gmp-integrity/src/lib.rs

pub mod alcoa_plus;
pub mod merkle_chain;
pub mod checksum;
pub mod tamper_detect;
pub mod snapshot;

pub struct IntegrityEngine {
    pub validator: AlcoaPlusValidator,
    pub merkle_chain: MerkleAuditChain,
    pub checksum_engine: ChecksumEngine,
    pub tamper_detector: TamperDetector,
}

impl IntegrityEngine {
    pub fn validate_entry(&self, entry: &AuditEntry) -> IntegrityResult<AlcoaPlusReport>;
    pub fn append_to_chain(&mut self, event: AuditEvent) -> IntegrityResult<MerkleAuditEntry>;
    pub fn verify_chain(&self) -> IntegrityResult<bool>;
    pub fn detect_tamper(&self) -> IntegrityResult<Vec<TamperDetection>>;
    pub fn generate_proof(&self, index: u64) -> IntegrityResult<MerkleProof>;
    pub fn create_snapshot(&mut self) -> IntegrityResult<ChainSnapshot>;
}
```

**Merkle Tree 结构**：

```
                    Root Hash
                   /         \
            H(0-3)            H(4-7)
           /      \           /      \
       H(0-1)   H(2-3)   H(4-5)   H(6-7)
       /   \    /   \    /   \    /   \
     H(0) H(1) H(2) H(3) H(4) H(5) H(6) H(7)
      │    │    │    │    │    │    │    │
     E0   E1   E2   E3   E4   E5   E6   E7
```

**依赖**：
- `sqlrustgo-types`
- `sqlrustgo-storage`
- `sha2` (SHA-256)
- `merkle` (Merkle Tree)

### 3.3 crates/hermes-runtime

**职责**：HermesAgent AI 运行时

```rust
// crates/hermes-runtime/src/lib.rs

pub mod agent;
pub mod tool;
pub mod workflow_bridge;
pub mod knowledge_bridge;
pub mod audit_bridge;

pub struct HermesRuntime {
    pub llm_client: Arc<dyn LlmClient>,
    pub tool_registry: Arc<ToolRegistry>,
    pub workflow_engine: Arc<GmpWorkflowEngine>,
    pub knowledge_base: Arc<dyn KnowledgeBase>,
    pub audit_logger: Arc<dyn AuditLogger>,
    pub config: HermesConfig,
}

pub trait GmpAgent: Send + Sync {
    fn name(&self) -> &str;
    async fn investigate_deviation(&self, ctx: &DeviationContext) -> AgentResult<DeviationDraft>;
    async fn generate_capa(&self, deviation: &Deviation) -> AgentResult<CapaSuggestion>;
    async fn review_batch(&self, batch_id: &str) -> AgentResult<BatchReviewDraft>;
    async fn assist_sop(&self, query: &str, device_id: &str) -> AgentResult<SopAssistance>;
}

pub struct ToolRegistry {
    pub tools: HashMap<String, Box<dyn AgentTool>>,
}

pub trait AgentTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, params: &serde_json::Value) -> AgentResult<serde_json::Value>;
}
```

**AI Agent 工具链**：

| 工具 | 描述 | 数据源 |
|------|------|--------|
| `query_device` | 查询设备信息 | Graph Engine |
| `query_batch` | 查询批次信息 | SQL Engine |
| `search_sop` | 搜索 SOP | Vector Engine |
| `search_regulation` | 搜索法规 | Vector Engine |
| `query_deviation_history` | 查询历史偏差 | Graph Engine |
| `query_calibration` | 查询校准记录 | SQL Engine |
| `create_deviation` | 创建偏差 | Workflow Engine |
| `create_capa` | 创建 CAPA | Workflow Engine |
| `sign_document` | 签署文档 | Regulatory Core |

**依赖**：
- `sqlrustgo-types`
- `sqlrustgo-regulatory-core`
- `async-trait`
- `serde`, `serde_json`
- `tokio`

### 3.4 crates/gmp-knowledge

**职责**：GMP 知识图谱

```rust
// crates/gmp-knowledge/src/lib.rs

pub mod schema;
pub mod loaders;
pub mod analytics;
pub mod regulations;
pub mod queries;
pub mod provenance;

pub struct GmpKnowledgeGraph {
    pub graph_store: Arc<dyn GraphStore>,
    pub schema: GmpSchema,
    pub analytics: GraphAnalytics,
}

pub struct GmpSchema {
    pub node_types: Vec<NodeType>,
    pub edge_types: Vec<EdgeType>,
    pub constraints: Vec<SchemaConstraint>,
}

pub struct NodeType {
    pub label: String,
    pub properties: Vec<PropertyDef>,
    pub indexes: Vec<IndexDef>,
}

// 预定义节点类型
impl GmpSchema {
    pub fn batch_type() -> NodeType;
    pub fn device_type() -> NodeType;
    pub fn sop_type() -> NodeType;
    pub fn operator_type() -> NodeType;
    pub fn capa_type() -> NodeType;
    pub fn deviation_type() -> NodeType;
    pub fn regulation_type() -> NodeType;
}
```

**图谱分析能力**：

```rust
pub struct GraphAnalytics {
    pub graph_store: Arc<dyn GraphStore>,
}

impl GraphAnalytics {
    pub fn root_cause_analysis(&self, deviation_id: &str) -> AnalyticsResult<Vec<RootCausePath>>;
    pub fn risk_propagation(&self, source_id: &str, depth: usize) -> AnalyticsResult<RiskPropagationTree>;
    pub fn regulation_mapping(&self, deviation_id: &str) -> AnalyticsResult<Vec<RegulationMapping>>;
    pub fn device_failure_prediction(&self, device_id: &str) -> AnalyticsResult<FailurePrediction>;
    pub fn batch_impact_analysis(&self, device_id: &str) -> AnalyticsResult<Vec<AffectedBatch>>;
}
```

**依赖**：
- `sqlrustgo-graph`
- `sqlrustgo-vector`
- `sqlrustgo-types`

### 3.5 crates/gmp-iot

**职责**：工业物联网连接

```rust
// crates/gmp-iot/src/lib.rs

pub mod opcua;
pub mod mqtt;
pub mod edge;
pub mod event_bus;
pub mod calibration;
pub mod environmental;

pub struct IotGateway {
    pub opcua_connector: Option<OpcUaConnector>,
    pub mqtt_collector: Option<MqttCollector>,
    pub edge_gateway: EdgeGateway,
    pub event_bus: Arc<EventBus>,
}

pub trait IotDataSource: Send + Sync {
    fn connect(&mut self) -> IotResult<()>;
    fn read(&self, point: &str) -> IotResult<DataValue>;
    fn subscribe(&mut self, point: &str, callback: Box<dyn Fn(DataValue) + Send>) -> IotResult<()>;
    fn disconnect(&mut self) -> IotResult<()>;
}
```

**Edge Gateway 离线同步**：

```
在线模式:
  IoT Sensor → MQTT → Edge Gateway → SQLRustGo Cloud

离线模式:
  IoT Sensor → MQTT → Edge Gateway → Local Store
                                    (缓冲)
在线恢复:
  Local Store → Sync Manager → SQLRustGo Cloud
                  (冲突解决)
```

**依赖**：
- `sqlrustgo-types`
- `sqlrustgo-storage`
- `sqlrustgo-regulatory-core`
- `tokio`
- `rumqttc` (MQTT)
- `opcua` (OPC UA)

### 3.6 crates/regulatory-ai

**职责**：AI 合规引擎

```rust
// crates/regulatory-ai/src/lib.rs

pub mod audit_ai;
pub mod regulation_parser;
pub mod inspection_simulator;
pub mod anomaly_detector;

pub struct ComplianceAI {
    pub auditor: ComplianceAuditor,
    pub regulation_parser: RegulationParser,
    pub inspection_simulator: InspectionSimulator,
    pub anomaly_detectors: Vec<Box<dyn AnomalyDetector>>,
}

pub trait AnomalyDetector: Send + Sync {
    fn anomaly_type(&self) -> &str;
    fn detect(&self, entries: &[MerkleAuditEntry]) -> AnomalyResult<Vec<Anomaly>>;
    fn confidence(&self) -> f64;
}
```

**异常检测器**：

| 检测器 | 类型 | 方法 |
|--------|------|------|
| `BackdatingDetector` | 数据回填 | 时间戳统计分析 |
| `TamperingDetector` | 数据篡改 | Merkle Proof 验证 |
| `SignatureAnomalyDetector` | 异常签名 | 签名模式分析 |
| `UnauthorizedDetector` | 越权操作 | 权限图遍历 |

**依赖**：
- `sqlrustgo-regulatory-core`
- `sqlrustgo-gmp-integrity`
- `sqlrustgo-gmp-knowledge`
- `sqlrustgo-hermes-runtime`

### 3.7 crates/plugin-sdk

**职责**：插件开发 SDK

```rust
// crates/plugin-sdk/src/lib.rs

pub mod plugin;
pub mod context;
pub mod registry;
pub mod lifecycle;

pub trait GmpPlugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    fn on_load(&mut self, ctx: &PluginContext) -> PluginResult<()>;
    fn on_event(&mut self, event: &GmpEvent) -> PluginResult<()>;
    fn on_unload(&mut self) -> PluginResult<()>;
}

pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub event_subscriptions: Vec<String>,
}

pub struct PluginContext {
    pub event_bus: Arc<EventBus>,
    pub audit_chain: Arc<MerkleAuditChain>,
    pub knowledge_graph: Arc<GmpKnowledgeGraph>,
    pub regulatory_kernel: Arc<RegulatoryKernel>,
    pub config: PluginConfig,
}
```

**依赖**：
- `sqlrustgo-types`
- `sqlrustgo-regulatory-core`

---

## 四、Event Sourcing 架构 (v4.0)

### 4.1 事件模型

```rust
pub struct DomainEvent {
    pub event_id: EventId,
    pub aggregate_id: AggregateId,
    pub aggregate_type: AggregateType,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: TrustedTimestamp,
    pub operator: String,
    pub metadata: EventMetadata,
}

pub enum AggregateType {
    Batch,
    Device,
    Deviation,
    Capa,
    Sop,
    Operator,
    Calibration,
}
```

### 4.2 事件存储

```
┌──────────────────────────────────────────────────────────┐
│                    Event Store                            │
│                                                          │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐         │
│  │ Append Only│  │ Merkle     │  │ Snapshot   │         │
│  │ WAL Log    │  │ Integrity  │  │ Store      │         │
│  └────────────┘  └────────────┘  └────────────┘         │
│                                                          │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐         │
│  │ Projection │  │ Subscription│  │ Replay     │         │
│  │ Builder    │  │ Manager    │  │ Engine     │         │
│  └────────────┘  └────────────┘  └────────────┘         │
└──────────────────────────────────────────────────────────┘
```

### 4.3 投影 (Projection)

```rust
pub trait Projection: Send + Sync {
    fn name(&self) -> &str;
    fn handle(&mut self, event: &DomainEvent) -> ProjectionResult<()>;
    fn query(&self, query: &ProjectionQuery) -> ProjectionResult<ProjectionResult>;
}

// 示例：批次状态投影
pub struct BatchStatusProjection {
    pub states: HashMap<String, BatchStatus>,
}

pub struct BatchStatus {
    pub batch_id: String,
    pub status: BatchPhase,
    pub parameters: HashMap<String, f64>,
    pub deviations: Vec<String>,
    pub signatures: Vec<SignatureRecord>,
    pub environmental: EnvironmentalSnapshot,
}
```

---

## 五、Mobile App 架构

### 5.1 技术选型

| 层 | 技术 | 说明 |
|----|------|------|
| UI | Flutter | 跨平台 (iOS/Android/Tablet) |
| 通信 | gRPC | 高性能 + 类型安全 |
| 离线 | SQLite + CRDT | 离线数据 + 冲突解决 |
| 推送 | FCM/APNs | 实时报警 |
| 扫码 | Camera + ML Kit | SOP/设备扫码 |

### 5.2 App 模块

```
lib/
├── core/
│   ├── grpc_client.dart      # gRPC 客户端
│   ├── offline_store.dart    # 离线存储
│   ├── auth_service.dart     # 认证服务
│   └── sync_manager.dart     # 同步管理
├── features/
│   ├── sop/                  # SOP 扫码查看
│   ├── signature/            # 电子签名
│   ├── deviation/            # 偏差上报
│   ├── ai_assistant/         # AI 助手
│   ├── batch_record/         # 批记录
│   └── environmental/        # 环境监测
└── main.dart
```

### 5.3 离线同步策略

```
在线 → 离线:
  1. 预加载当前用户相关 SOP/设备/批次数据
  2. 切换到本地 SQLite 存储
  3. 操作记录写入本地 WAL

离线 → 在线:
  1. 检测网络恢复
  2. 本地 WAL → CRDT Merge
  3. 上传合并后的操作
  4. 下载服务端更新
```

---

## 六、安全架构

### 6.1 安全层次

```
┌─────────────────────────────────────────────┐
│              应用安全                         │
│  RBAC + RLS + 动态脱敏 + 审计日志            │
├─────────────────────────────────────────────┤
│              通信安全                         │
│  TLS 1.3 + mTLS (节点间) + 证书轮换          │
├─────────────────────────────────────────────┤
│              数据安全                         │
│  AES-256 加密 + SHA-256 校验 + HSM 密钥管理  │
├─────────────────────────────────────────────┤
│              审计安全                         │
│  Merkle Chain 防篡改 + 电子签名 + 可信时间戳  │
└─────────────────────────────────────────────┘
```

### 6.2 电子签名安全

```
签名流程:
  1. 用户输入用户名 + 密码
  2. 系统发送 OTP 到注册设备
  3. 用户输入 OTP
  4. 系统验证 → 生成签名
  5. 签名写入 Merkle Chain
  6. HSM 存储私钥

签名验证:
  1. 从 Merkle Chain 读取签名
  2. 验证 Merkle Proof
  3. 验证签名公钥
  4. 验证时间戳
  5. 验证签名含义 (approval/review/execution)
```

---

## 七、部署架构

### 7.1 单工厂部署

```
┌─────────────────────────────────────────┐
│            Factory Server               │
│  ┌─────────────────────────────────┐    │
│  │     SQLRustGo GMP-OS            │    │
│  │  ┌───────┐ ┌───────┐ ┌───────┐ │    │
│  │  │RegCore│ │Hermes │ │ IoT   │ │    │
│  │  └───────┘ └───────┘ └───────┘ │    │
│  │  ┌───────┐ ┌───────┐ ┌───────┐ │    │
│  │  │Graph  │ │Vector │ │Audit  │ │    │
│  │  └───────┘ └───────┘ └───────┘ │    │
│  └─────────────────────────────────┘    │
│  ┌─────────────────────────────────┐    │
│  │     Edge Gateway                │    │
│  │  OPC UA │ MQTT │ Local Store    │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
         │                │
    ┌────┘                └────┐
    ▼                          ▼
┌────────┐              ┌────────────┐
│Mobile  │              │PLC/SCADA/  │
│App     │              │Sensor      │
└────────┘              └────────────┘
```

### 7.2 多工厂部署 (SaaS)

```
┌─────────────────────────────────────────────┐
│              Cloud Control Plane             │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐  │
│  │ API       │ │ Tenant    │ │ Plugin    │  │
│  │ Gateway   │ │ Manager   │ │ Registry  │  │
│  └───────────┘ └───────────┘ └───────────┘  │
└──────────────────────┬──────────────────────┘
                       │
        ┌──────────────┼──────────────┐
        ▼              ▼              ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│ Factory A    │ │ Factory B    │ │ Factory C    │
│ (Shard 1)    │ │ (Shard 2)    │ │ (Shard 3)    │
└──────────────┘ └──────────────┘ └──────────────┘
```

---

## 八、Crate 依赖关系

### 8.1 GMP-OS Crate 依赖图

```
                    ┌─────────────────┐
                    │  api-gateway    │
                    └────────┬────────┘
                             │
        ┌────────────────────┼────────────────────┐
        ▼                    ▼                    ▼
┌───────────────┐  ┌───────────────┐  ┌───────────────┐
│ hermes-runtime│  │regulatory-core│  │  gmp-iot      │
└───────┬───────┘  └───────┬───────┘  └───────┬───────┘
        │                  │                   │
        ▼                  ▼                   │
┌───────────────┐  ┌───────────────┐           │
│gmp-knowledge  │  │gmp-integrity  │           │
└───────┬───────┘  └───────┬───────┘           │
        │                  │                   │
        ▼                  ▼                   ▼
┌───────────────────────────────────────────────────────┐
│                  SQLRustGo Core                        │
│  graph │ vector │ storage │ transaction │ types        │
└───────────────────────────────────────────────────────┘
```

### 8.2 新增 Crate 依赖矩阵

| Crate | 依赖 |
|-------|------|
| `regulatory-core` | types, storage, transaction, gmp-integrity |
| `gmp-integrity` | types, storage |
| `hermes-runtime` | types, regulatory-core, graph, vector |
| `gmp-knowledge` | graph, vector, types |
| `gmp-iot` | types, storage, regulatory-core |
| `regulatory-ai` | regulatory-core, gmp-integrity, gmp-knowledge, hermes-runtime |
| `plugin-sdk` | types, regulatory-core |
| `api-gateway` | regulatory-core, hermes-runtime, gmp-iot |

---

## 九、与现有模块的迁移映射

| 现有模块 | GMP-OS 目标 | 迁移策略 | 版本 |
|----------|------------|---------|------|
| `gmp::audit` | `regulatory-core::audit` | 直接迁移 | v3.4.0 |
| `gmp::audit_chain` | `gmp-integrity::merkle_chain` | 升级为 Merkle | v3.4.0 |
| `gmp::signature` | `regulatory-core::signature` | 迁移 + OTP | v3.4.0 |
| `gmp::evidence` | `regulatory-core::evidence` | 迁移 + ALCOA+ | v3.4.0 |
| `gmp::correction` | `regulatory-core::correction` | 直接迁移 | v3.4.0 |
| `gmp::document` | `regulatory-core::document` | 直接迁移 | v3.4.0 |
| `gmp::workflow` | `regulatory-core::workflow` | 升级为事件驱动 | v3.4.0 |
| `gmp::provenance` | `gmp-knowledge::provenance` | 迁移至知识图谱 | v3.6.0 |
| `gmp::hsm` | `regulatory-core::hsm` | 直接迁移 | v3.4.0 |
| `gmp::mobile` | Mobile App (Flutter) | 独立项目 | v3.4.0 |
| `gmp::sop` | `gmp-knowledge::sop` | 迁移至知识图谱 | v3.6.0 |
| `gmp::calibration` | `gmp-iot::calibration` | 迁移至 IoT 模块 | v3.7.0 |

---

## 十、变更日志

| 日期 | 版本 | 变更 | 作者 |
|------|------|------|------|
| 2026-05-18 | v1.0 | 初始版本 | hermes-agent |

---

*本文档定义了 SQLRustGo GMP-OS 的完整架构设计，覆盖从 v3.4.0 到 v4.0.0 的所有核心模块。*
