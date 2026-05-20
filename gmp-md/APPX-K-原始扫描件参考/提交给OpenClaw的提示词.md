# 提交给 OpenClaw 的提示词

## Phase 2 新增文档导入提示词

---

### 提示词内容

```
请将以下12份新增GMP文档导入到OpenClaw的知识库系统：

## Phase 2 新增文档清单

### 1. 质量管理体系 (3份)
1. /home/openclaw/gmp_docs/phase2/qms/退货管理规程.md
   - 文件编号: SP-QMS-015
   - GMP依据: 第6条
   - 说明: 退货申请、审核、接收、检验、处置流程

2. /home/openclaw/gmp_docs/phase2/qms/产品质量回顾分析规程.md
   - 文件编号: SP-QMS-016
   - GMP依据: 第8条
   - 说明: 年度产品质量回顾分析、趋势分析、CAPA触发

3. /home/openclaw/gmp_docs/phase2/qms/部门职责规定.md
   - 文件编号: SP-ORG-001
   - GMP依据: 第19条
   - 说明: 公司组织架构、各部门职责分工

### 2. 机构与人员 (1份)
4. /home/openclaw/gmp_docs/phase2/org/健康检查管理规程.md
   - 文件编号: SP-HR-002
   - GMP依据: 第31条
   - 说明: 入职体检、在岗定期体检、职业健康监护

### 3. 厂房与设施 (3份)
5. /home/openclaw/gmp_docs/phase2/facilities/压差监控规程.md
   - 文件编号: SP-FAC-010
   - GMP依据: 第38条
   - 说明: 洁净区压差监控标准、监控频率、异常处理

6. /home/openclaw/gmp_docs/phase2/facilities/温湿度控制规程.md
   - 文件编号: SP-FAC-011
   - GMP依据: 第38条
   - 说明: 洁净区及仓库温湿度控制标准、调控设备操作

7. /home/openclaw/gmp_docs/phase2/facilities/水系统日常监测规程.md
   - 文件编号: SP-FAC-006
   - GMP依据: 第65条
   - 说明: 纯化水/注射用水日常监测、取样、检验、异常处理

### 4. 物料与产品 (1份)
8. /home/openclaw/gmp_docs/phase2/materials/物料验收规程.md
   - 文件编号: SP-MAT-001
   - GMP依据: 第83条
   - 说明: 原料、辅料、包装材料验收流程、拒收标准

### 5. 生产管理 (1份)
9. /home/openclaw/gmp_docs/phase2/production/防混淆管理规程.md
   - 文件编号: SP-PROD-018
   - GMP依据: 第123条
   - 说明: 区域隔离、标识管理、关键操作复核、清场管理

### 6. 质量控制 (3份)
10. /home/openclaw/gmp_docs/phase2/qc/QC实验室管理规程.md
    - 文件编号: SP-QC-001
    - GMP依据: 第125条
    - 说明: 实验室布局、人员要求、仪器设备、试剂管理

11. /home/openclaw/gmp_docs/phase2/qc/检验记录管理规程.md
    - 文件编号: SP-QC-005
    - GMP依据: 第145条
    - 说明: 记录填写规范、审核流程、保存期限、电子记录

12. /home/openclaw/gmp_docs/phase2/qc/产品质量档案管理规程.md
    - 文件编号: SP-QC-012
    - GMP依据: 第152条
    - 说明: 产品质量档案建立、维护、使用、销毁

## 导入要求

1. 将上述12份文档添加到OpenClaw的向量数据库
2. 更新OpenClaw的qmd索引（如果使用qmd格式）
3. 确保文档能被以下查询正确检索：
   - "退货处理流程"
   - "产品质量回顾分析"
   - "部门职责"
   - "健康检查"
   - "压差监控"
   - "温湿度控制"
   - "水系统监测"
   - "物料验收"
   - "防混淆措施"
   - "QC实验室管理"
   - "检验记录规范"
   - "产品质量档案"

## 文档分类标签

建议使用以下分类标签：
- 质量管理体系: qms, quality_management
- 机构与人员: org, personnel
- 厂房与设施: facilities, environment
- 物料与产品: materials
- 生产管理: production
- 质量控制: qc, quality_control
```

---

## 执行命令参考

```bash
# 1. 将文档复制到OpenClaw的导入目录
cp /home/openclaw/gmp_docs/phase2/qms/退货管理规程.md /path/to/openclaw/import/
cp /home/openclaw/gmp_docs/phase2/qms/产品质量回顾分析规程.md /path/to/openclaw/import/
cp /home/openclaw/gmp_docs/phase2/qms/部门职责规定.md /path/to/openclaw/import/
cp /home/openclaw/gmp_docs/phase2/org/健康检查管理规程.md /path/to/openclaw/import/
cp /home/openclaw/gmp_docs/phase2/facilities/压差监控规程.md /path/to/openclaw/import/
cp /home/openclaw/gmp_docs/phase2/facilities/温湿度控制规程.md /path/to/openclaw/import/
cp /home/openclaw/gmp_docs/phase2/facilities/水系统日常监测规程.md /path/to/openclaw/import/
cp /home/openclaw/gmp_docs/phase2/materials/物料验收规程.md /path/to/openclaw/import/
cp /home/openclaw/gmp_docs/phase2/production/防混淆管理规程.md /path/to/openclaw/import/
cp /home/openclaw/gmp_docs/phase2/qc/QC实验室管理规程.md /path/to/openclaw/import/
cp /home/openclaw/gmp_docs/phase2/qc/检验记录管理规程.md /path/to/openclaw/import/
cp /home/openclaw/gmp_docs/phase2/qc/产品质量档案管理规程.md /path/to/openclaw/import/

# 2. 执行OpenClaw的导入脚本
python /path/to/openclaw/scripts/import_documents.py --directory /path/to/openclaw/import/

# 3. 验证导入结果
python /path/to/openclaw/scripts/verify_import.py --check-documents 12
```

---

## 新增文档汇总表

| 序号 | 文件编号 | 文件名称 | GMP条款 | 分类 |
|------|----------|----------|---------|------|
| 1 | SP-QMS-015 | 退货管理规程 | 第6条 | 质量管理体系 |
| 2 | SP-QMS-016 | 产品质量回顾分析规程 | 第8条 | 质量管理体系 |
| 3 | SP-ORG-001 | 部门职责规定 | 第19条 | 机构与人员 |
| 4 | SP-HR-002 | 健康检查管理规程 | 第31条 | 机构与人员 |
| 5 | SP-FAC-010 | 压差监控规程 | 第38条 | 厂房与设施 |
| 6 | SP-FAC-011 | 温湿度控制规程 | 第38条 | 厂房与设施 |
| 7 | SP-FAC-006 | 水系统日常监测规程 | 第65条 | 厂房与设施 |
| 8 | SP-MAT-001 | 物料验收规程 | 第83条 | 物料与产品 |
| 9 | SP-PROD-018 | 防混淆管理规程 | 第123条 | 生产管理 |
| 10 | SP-QC-001 | QC实验室管理规程 | 第125条 | 质量控制 |
| 11 | SP-QC-005 | 检验记录管理规程 | 第145条 | 质量控制 |
| 12 | SP-QC-012 | 产品质量档案管理规程 | 第152条 | 质量控制 |

---

## 预期效果

导入完成后，OpenClaw应能够正确回答以下GMP相关问题：

| 问题类型 | 示例问题 |
|----------|----------|
| 退货管理 | "药品退货的处理流程是什么？" |
| 质量回顾 | "产品质量回顾分析包括哪些内容？" |
| 部门职责 | "生产部和质量部的职责分别是什么？" |
| 健康检查 | "员工入职体检有哪些项目？" |
| 压差监控 | "洁净区压差标准是多少？如何监控？" |
| 温湿度控制 | "洁净区温湿度标准是什么？" |
| 水系统监测 | "纯化水日常监测项目有哪些？" |
| 物料验收 | "原料药验收需要检查哪些内容？" |
| 防混淆 | "如何防止生产过程中的混淆？" |
| QC实验室 | "QC实验室有哪些管理要求？" |
| 检验记录 | "检验记录填写有哪些规范要求？" |
| 产品档案 | "产品质量档案包括哪些内容？" |

---

**提交人**: 高小彝 (HermesAgent)
**提交日期**: 2026-04-12
**文档总数**: 12份
