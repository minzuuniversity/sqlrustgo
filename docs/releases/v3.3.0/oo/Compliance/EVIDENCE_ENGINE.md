# OO-CE2: Evidence Engine

> **版本**: v1.0
> **日期**: 2026-05-18
> **基于**: v3.3.0
> **维护人**: hermes-agent
> **Issue**: #1239
> **状态**: 新建设计

---

## 一、概述

### 1.1 目标

自动生成 FDA 审计证据包，支持一键导出完整审计材料用于监管检查。

### 1.2 核心理念

```
Evidence Engine = Automated Collection + Chain Verification + Multi-Format Export + Cryptographic Proof
```

### 1.3 输出格式

| 格式 | 用途 |
|------|------|
| JSON | 机器可读，API 集成 |
| PDF | 人工审核，监管提交 |
| XML | 监管机构交换格式 |
| ZIP | 完整证据包归档 |

---

## 二、架构设计

### 2.1 组件关系

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       Evidence Engine                                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐    ┌──────────────┐    ┌────────────────────────────┐  │
│  │   Evidence   │───▶│    Chain     │───▶│   Export Generator         │  │
│  │   Collector  │    │   Verifier   │    │  (JSON/PDF/XML/ZIP)        │  │
│  └──────────────┘    └──────────────┘    └────────────────────────────┘  │
│          │                  │                          │                   │
│          ▼                  ▼                          ▼                   │
│  ┌──────────────┐    ┌──────────────┐    ┌────────────────────────────┐  │
│  │  Manifest    │    │  Hash Chain  │    │   Signature Generator      │  │
│  │  Builder    │    │  Validator   │    │   (ECDSA P-256)            │  │
│  └──────────────┘    └──────────────┘    └────────────────────────────┘  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 组件说明

| 组件 | 说明 | 位置 |
|------|------|------|
| Evidence Collector | 收集审计证据 | `crates/evidence-collector/` |
| Chain Verifier | 验证证据链完整性 | `crates/evidence-verifier/` |
| Export Generator | 生成多格式导出 | `crates/evidence-exporter/` |
| Signature Generator | 生成数字签名 | `crates/signature/` |

---

## 三、证据包结构

### 3.1 Audit Package 结构

```json
{
  "audit_package": {
    "version": "1.0",
    "package_id": "pkg_2026_001_001",
    "generated_at": "2026-05-18T12:00:00Z",
    "generated_by": "evidence-engine/1.0",

    "batch_info": {
      "batch_id": "2026-001",
      "product_name": "Insulin Glargine",
      "facility": "F001",
      "manufacturing_date": "2026-05-01",
      "expiry_date": "2028-05-01"
    },

    "manifest": {
      "total_records": 1523,
      "total_signatures": 47,
      "total_workflows": 12,
      "evidence_items": [
        {
          "type": "batch_record",
          "count": 1,
          "hash": "sha256:abc123..."
        },
        {
          "type": "signature",
          "count": 47,
          "hash": "sha256:def456..."
        },
        {
          "type": "workflow",
          "count": 12,
          "hash": "sha256:ghi789..."
        }
      ]
    },

    "records": [
      {
        "record_id": "rec_001",
        "type": "weighing",
        "timestamp": "2026-05-01T08:30:00Z",
        "operator": {
          "id": "op_001",
          "name": "John Smith",
          "role": "weighing_operator"
        },
        "data": {
          "ingredient": "Insulin Glargine",
          "quantity": "100.5g",
          "balance_id": "BAL_001",
          "verification": " calibrated"
        },
        "signature": {
          "id": "sig_001",
          "type": "electronic",
          "hash": "sha256:xxx..."
        },
        "hash": "sha256:record_hash..."
      }
    ],

    "signatures": [
      {
        "signature_id": "sig_001",
        "signer": {
          "id": "op_001",
          "name": "John Smith",
          "role": "quality_reviewer"
        },
        "timestamp": "2026-05-01T10:30:00Z",
        "meaning": "I confirm this record is accurate and complete",
        "hash": "sha256:sig_hash...",
        "public_key_id": "pk_001",
        "algorithm": "ECDSA_P256_SHA256"
      }
    ],

    "workflows": [
      {
        "workflow_id": "wf_001",
        "type": "batch_release",
        "current_state": "approved",
        "states": [
          {
            "state": "pending",
            "entered_at": "2026-05-01T08:00:00Z"
          },
          {
            "state": "under_review",
            "entered_at": "2026-05-01T12:00:00Z"
          },
          {
            "state": "approved",
            "entered_at": "2026-05-02T09:00:00Z"
          }
        ]
      }
    ],

    "hash_proof": {
      "algorithm": "SHA256",
      "root_hash": "sha256:root_hash_value...",
      "tree_structure": "merkle_tree",
      "leaves_count": 1523
    },

    "chain_verification": {
      "status": "valid",
      "verification_timestamp": "2026-05-18T12:05:00Z",
      "checks_performed": [
        "record_integrity",
        "signature_validity",
        "workflow_completeness",
        "hash_chain_continuity",
        "timestamp_ordering"
      ],
      "result": "all_passed"
    },

    "digital_signature": {
      "signed_at": "2026-05-18T12:10:00Z",
      "signer": "evidence_engine",
      "signature_value": "base64_encoded_signature...",
      "certificate_thumbprint": "sha256:cert_thumbprint..."
    }
  }
}
```

---

## 四、执行流程

### 4.1 证据包生成流程

```
┌─────────────────────────────────────────────────────────────────┐
│               Evidence Package Generation Flow                     │
└─────────────────────────────────────────────────────────────────┘

Request: GET /api/v1/audit-packages/{batch_id}
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  1. Evidence Collection                                          │
│     - Query records for batch_id                                │
│     - Gather all signatures                                     │
│     - Collect workflow history                                   │
│     - Fetch calibration records                                 │
│     - Retrieve deviation records                                │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  2. Manifest Building                                            │
│     - Count evidence items by type                              │
│     - Compute individual hashes                                  │
│     - Build Merkle tree                                          │
│     - Compute root hash                                         │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  3. Chain Verification                                           │
│     - Verify record hashes                                       │
│     - Validate signature chains                                  │
│     - Check workflow state transitions                           │
│     - Confirm timestamp ordering                                 │
│     - Verify hash chain continuity                              │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  4. Format Generation                                            │
│     - Generate JSON manifest                                     │
│     - Generate PDF report                                        │
│     - Generate XML for regulatory exchange                       │
│     - Package all in ZIP                                        │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  5. Digital Signature                                           │
│     - Sign package with ECDSA private key                        │
│     - Attach signature and certificate                           │
│     - Make tamper-evident                                        │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
    Response: AuditPackage
```

### 4.2 Chain Verification Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                  Chain Verification Flow                         │
└─────────────────────────────────────────────────────────────────┘

Start Verification
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  Check 1: Record Integrity                                       │
│  - Recompute each record's hash                                  │
│  - Compare with stored hash                                      │
│  - FAIL if mismatch                                              │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  Check 2: Signature Validity                                     │
│  - Verify each signature with public key                        │
│  - Check certificate chain                                        │
│  - Verify against timestamp authority                           │
│  - FAIL if invalid                                              │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  Check 3: Workflow Completeness                                  │
│  - Verify all required states present                           │
│  - Check state transition validity                               │
│  - FAIL if missing required state                               │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  Check 4: Hash Chain Continuity                                  │
│  - Verify record sequence hash chain                           │
│  - Check no gaps in LSN                                         │
│  - FAIL if broken chain                                         │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  Check 5: Timestamp Ordering                                     │
│  - Verify chronological order                                   │
│  - Check no future timestamps                                    │
│  - FAIL if ordering violated                                    │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
    ┌──────────────┐
    │ All Checks   │
    │   PASSED     │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ Return:     │
    │ valid        │
    └──────────────┘
```

---

## 五、PDF 报告结构

### 5.1 章节组织

```
FDA Audit Package - Batch 2026-001
═══════════════════════════════════════════════════════════════════

Cover Page
├── Company Logo
├── Document Title
├── Batch ID and Product
├── Date Range
└── Digital Signature

Table of Contents
├── 1. Executive Summary
├── 2. Batch Information
├── 3. Manufacturing Records
├── 4. Quality Control Results
├── 5. Electronic Signatures
├── 6. Workflow History
├── 7. Deviation Report
├── 8. Chain of Custody
└── 9. Verification Certificates

1. Executive Summary
├── Package ID
├── Total Records
├── Total Signatures
├── Verification Status
└── Generation Timestamp

2. Batch Information
├── Product Details
├── Manufacturing Facility
├── Date of Manufacture
├── Batch Size
└── Equipment List

3-8. Detailed Records
├── Each record with:
│   ├── Timestamp
│   ├── Operator Info
│   ├── Data Values
│   ├── Signature
│   └── Verification Status

9. Verification Certificates
├── Hash Chain Certificate
├── Digital Signature Certificate
└── Compliance Declaration
```

---

## 六、验收标准

### 6.1 功能验收

| 检查项 | 命令 | 标准 |
|--------|------|------|
| JSON 导出 | `evidence-engine export --batch-id 2026-001 --format json` | 生成有效 JSON |
| PDF 导出 | `evidence-engine export --batch-id 2026-001 --format pdf` | 生成可读 PDF |
| ZIP 打包 | `evidence-engine export --batch-id 2026-001 --format zip` | 完整包 |
| Chain 验证 | `evidence-engine verify --package-id pkg_001` | 返回 valid/invalid |

### 6.2 完整性验收

```bash
# 生成证据包
evidence-engine generate --batch-id 2026-001

# 验证证据包
evidence-engine verify --package-id 2026-001 --signature sig_xxx

# Expected output:
# {
#   "status": "valid",
#   "checks_passed": 5,
#   "checks_failed": 0,
#   "verification_timestamp": "2026-05-18T12:05:00Z"
# }
```

---

## 七、相关文档

- `docs/releases/v3.3.0/TRUST_INFRASTRUCTURE_STRATEGY.md` - 战略定位
- `crates/evidence-collector/` - 证据收集器
- `crates/evidence-exporter/` - 导出器
- `crates/evidence-verifier/` - 验证器

---

*本文档由 hermes-agent 生成*
*版本 1.0 - 2026-05-18*
