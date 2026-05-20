# GMP 合规文档数据集 — 脱敏版

> **版本**: v1.0 | **日期**: 2026-05-20 | **来源**: GMP-Platform gmp-md

## 概述

本目录包含 1696 份中国 GMP 合规文档，覆盖药品生产质量管理规范的完整领域。
文档已通过 7 类脱敏处理，可用于：
- 公开发布到云端
- 学术研究数据集
- GMP 合规 AI Agent 训练
- 合规性内审检索系统开发

## 脱敏范围

| 类别 | 脱敏方式 | 示例 |
|------|----------|------|
| 企业名称 | 占位符替换 | [COMPANY-NAME-PHARMA] |
| 企业代码 | 占位符替换 | [ENT-CODE] |
| 人名 | 占位符替换 | [NAME-A/NAME-B/NAME-C/NAME-D] |
| 产品名称 | 占位符替换 | [PRODUCT-A-CAPSULE], [PRODUCT-B-TINCTURE] |
| 中药材名 | 占位符替换 | [HERB-A] ~ [HERB-G] |
| 注册批号 | 占位符替换 | [REG-NO-SAMPLE], [REG-NO-PREFIX] |
| 地址/电话/身份证 | 占位符替换 | [ADDRESS-FACILITY], [ID-NUMBER] |

## 目录结构

```
gmp-md-masking/
├── CH01-文件管理与数据完整性/     # GMP 第1章 文件管理
├── CH02-质量管理体系/             # GMP 第2章 质量管理体系
├── CH03-机构与人员/               # GMP 第3章 机构与人员
├── CH04-厂房与设施/               # GMP 第4章 厂房与设施
├── CH05-设备管理/                 # GMP 第5章 设备
├── CH06-物料与产品/               # GMP 第6章 物料与产品
├── CH07-确认与验证/               # GMP 第7章 确认与验证
├── CH08-生产管理/                 # GMP 第8章 生产管理
├── CH09-委托生产与委托检验/       # GMP 第9章 委托
├── APPX-A-GMP法规条款与解读/      # 中国GMP 2010版 附录
├── APPX-B-变更控制与企业补充规程/ # 企业补充规程
├── APPX-C~L/                      # 其他附录
└── MASKING-MAP.json               # 脱敏映射表（切勿公开）
```

## 技术规格

- 格式: Markdown (.md)
- 编码: UTF-8
- 文档数量: 1696 份
- GMP 章节: 9 章 + 12 附录
- 覆盖: SOP、GUI、REC、POL、VAL、BPR、SOP、AGR、ROOT 等

## 使用协议

本数据集仅供学术研究和系统开发参考。脱敏占位符保留了文档结构和领域知识，
但不包含任何真实企业信息。映射表 (MASKING-MAP.json) 仅保存在本地私有仓库。

## 引用

如需在学术论文中引用本数据集，请使用：
```
GMP-Platform Team. (2026). GMP Compliance Document Dataset (v1.0) [Data set].
```
