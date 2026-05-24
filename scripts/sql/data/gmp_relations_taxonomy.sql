-- ============================================================
-- GMP Relation Templates & Taxonomy Rules Data
-- ============================================================
-- Source: scripts/run_layer0_pipeline.py RELATION_TEMPLATES + TAXONOMY_*_PATTERNS
-- Purpose: Seed relation_templates and taxonomy_rules tables
-- Usage: sqlite3 data/gmp.db < scripts/sql/data/gmp_relations_taxonomy.sql
-- ============================================================

-- ============================================================
-- Section 1: Relation Templates (from RELATION_TEMPLATES)
-- ============================================================

INSERT INTO relation_templates (pattern, relation_type, src_type_hint, dst_type_hint) VALUES
('(\\w{2,20})触发([A-Z]{2,}-?\\d+)', 'TRIGGERS', 'Deviation', 'CAPA'),
('(\\w{2,20})引发([A-Z]{2,}-?\\d+)', 'TRIGGERS', 'Deviation', 'CAPA'),
('(\\w{2,20})导致([A-Z]{2,}-?\\d+)', 'TRIGGERS', 'Deviation', 'CAPA'),
('审计发现[的]?(\\w{2,10})启动([A-Z]{2,}-?\\d+)', 'GENERATES', 'AuditFinding', 'CAPA'),
('(\\w{2,10})需采取([A-Z]{2,}-?\\d+)', 'GENERATES', 'AuditFinding', 'CAPA'),
('(\\w{2,10})需进行(IQ|OQ|PQ|DQ|验证|确认)', 'REQUIRES', 'Equipment', 'Validation'),
('(\\w{2,10})设备需要(IQ|OQ|PQ)', 'REQUIRES', 'Equipment', 'Validation'),
('(GMP第?\\d+条)[规]?定[了]?(\\w{2,10})', 'MANDATES', 'Regulation', 'SOP'),
('(\\w{2,10})由(质量部|生产部|工程部|仓储部)管理', 'MANAGED_BY', 'SOP', 'Department'),
('(\\w{2,10})属于(第[一二三四五六七八九十\\d]+章)', 'BELONGS_TO', 'SOP', 'Chapter');

-- ============================================================
-- Section 2: Taxonomy Topic Patterns (from TAXONOMY_TOPIC_PATTERNS)
-- ============================================================

INSERT INTO taxonomy_rules (dimension, pattern, value, priority) VALUES
('topic', '偏差|偏离|异常', '偏差', 10),
('topic', '变更控制|变更申请|变更管理', '变更控制', 10),
('topic', '验证方案|验证报告|确认方案|IQ|OQ|PQ', '验证', 10),
('topic', '设备管理|设备维护|设备校准', '设备', 10),
('topic', '物料管理|物料控制|仓储管理', '物料', 10),
('topic', '文件管理|文档管理|SOP管理', '文件', 10),
('topic', '人员培训|培训管理|资质确认', '人员', 10),
('topic', '厂房设施|洁净区|环境监控|HVAC', '厂房', 10);

-- ============================================================
-- Section 3: Taxonomy DocType Patterns (from TAXONOMY_DOCTYPE_PATTERNS)
-- ============================================================

INSERT INTO taxonomy_rules (dimension, pattern, value, priority) VALUES
('doc_type', 'SOP[-_]?\\d+|标准操作规程', 'SOP', 20),
('doc_type', '管理规程|管理文件', '管理规程', 15),
('doc_type', '批生产记录|批记录|检验记录|培训记录', '记录', 15),
('doc_type', '验证报告|确认报告|调查报告|审计报告', '报告', 15),
('doc_type', '验证方案|确认方案|培训计划|审计计划', '方案', 15),
('doc_type', 'GMP[第]?\\d*[条章]?|药典', '法规', 10);

-- ============================================================
-- Section 4: Taxonomy Risk Patterns (from TAXONOMY_RISK_PATTERNS)
-- ============================================================

INSERT INTO taxonomy_rules (dimension, pattern, value, priority) VALUES
('risk_level', '重大偏差|关键设备|关键工艺参数|无菌|高风险', '高', 20),
('risk_level', '一般偏差|重要|中等', '中', 15),
('risk_level', '微小偏差|低风险|次要', '低', 15);

-- ============================================================
-- Section 5: Taxonomy Department Patterns (from TAXONOMY_DEPT_PATTERNS)
-- ============================================================

INSERT INTO taxonomy_rules (dimension, pattern, value, priority) VALUES
('department', '质量部|质量管理部|QA', '质量部', 20),
('department', '生产部|生产管理部|生产车间', '生产部', 20),
('department', '工程部|设备工程部', '工程部', 20),
('department', '仓储部|仓库', '仓储部', 20),
('department', '采购部', '采购部', 20);
