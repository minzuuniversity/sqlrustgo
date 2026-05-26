# Deviation Investigator Agent

> **Version**: 1.0
> **Date**: 2026-05-26
> **Issue**: #1361
> **Status**: Planning

## 一、功能描述

AI 偏差调查助手，自然语言查询审计链，AI 直接定位根因。

## 二、用户交互流程

1. 用户输入偏差描述（自然语言）
2. Agent 从 Audit API 检索相关审计链
3. Agent 分析偏差链，识别根因
4. Agent 生成偏差报告，包含纠正措施建议

## 三、技术方案

### 3.1 输入

```json
{
  "deviation_description": "批次 #12345 在步骤 3 温度超标",
  "batch_id": "12345",
  "time_range": "2026-05-01 ~ 2026-05-26"
}
```

### 3.2 输出

```json
{
  "root_cause": "设备 #789 温控传感器故障",
  "evidence": ["sensor_log_456", "maintenance_record_123"],
  "corrective_actions": ["更换传感器", "重新校准温控系统"],
  "confidence": 0.92
}
```

## 四、验收标准

- 自然语言查询审计链，返回引用证据
- 偏差报告生成，提供纠正措施建议
- G8 Gate: AI Deviation Accuracy ≥ 90%

## 五、依赖

- GMP Audit API
- Ollama 集成 (#1364)
- GMP Retrieval v3 (#1363)
