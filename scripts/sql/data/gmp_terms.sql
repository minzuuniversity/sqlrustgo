-- ============================================================
-- GMP Terms Data Initialization
-- ============================================================
-- Source: scripts/run_layer0_pipeline.py ENTITY_PATTERNS + GMP_TERM_PATTERNS
-- Purpose: Seed the gmp_terms, term_synonyms, term_hierarchy tables
-- Usage: sqlite3 data/gmp.db < scripts/sql/data/gmp_terms.sql
-- ============================================================

-- ============================================================
-- Section 1: Canonical GMP Terms (from ENTITY_PATTERNS)
-- ============================================================

-- Deviation (偏差) entities
INSERT INTO gmp_terms (term_id, canonical_term, term_type, synonyms, hypernyms, hyponyms, gmp_chapter, entity_type) VALUES
(1, '偏差', 'entity', '["偏离","异常"]', '质量事件', NULL, '第七章', 'Deviation');

INSERT INTO term_synonyms (term_id, synonym) VALUES
(1, '偏离'), (1, '异常'), (1, '偏差报告');

INSERT INTO term_hierarchy (term_id, parent_term, hierarchy_type) VALUES
(1, '质量事件', 'hypernym');

-- Major deviation
INSERT INTO gmp_terms (term_id, canonical_term, term_type, synonyms, hypernyms, hyponyms, gmp_chapter, entity_type) VALUES
(2, '重大偏差', 'entity', '["严重偏差","重大偏离"]', '偏差', NULL, '第七章', 'Deviation');

INSERT INTO term_synonyms (term_id, synonym) VALUES
(2, '严重偏差'), (2, '重大偏离');

INSERT INTO term_hierarchy (term_id, parent_term, hierarchy_type) VALUES
(2, '偏差', 'hyponym');

-- General deviation
INSERT INTO gmp_terms (term_id, canonical_term, term_type, synonyms, hypernyms, hyponyms, gmp_chapter, entity_type) VALUES
(3, '一般偏差', 'entity', '["普通偏差","一般偏离"]', '偏差', NULL, '第七章', 'Deviation');

INSERT INTO term_synonyms (term_id, synonym) VALUES
(3, '普通偏差'), (3, '一般偏离');

INSERT INTO term_hierarchy (term_id, parent_term, hierarchy_type) VALUES
(3, '偏差', 'hyponym');

-- Minor deviation
INSERT INTO gmp_terms (term_id, canonical_term, term_type, synonyms, hypernyms, hyponyms, gmp_chapter, entity_type) VALUES
(4, '微小偏差', 'entity', '["轻微偏差","次要偏离"]', '偏差', NULL, '第七章', 'Deviation');

INSERT INTO term_synonyms (term_id, synonym) VALUES
(4, '轻微偏差'), (4, '次要偏离');

INSERT INTO term_hierarchy (term_id, parent_term, hierarchy_type) VALUES
(4, '偏差', 'hyponym');

-- CAPA
INSERT INTO gmp_terms (term_id, canonical_term, term_type, synonyms, hypernyms, hyponyms, gmp_chapter, entity_type) VALUES
(5, 'CAPA', 'entity', '["纠正措施","预防措施","整改措施"]', '质量保证', NULL, '第八章', 'CAPA');

INSERT INTO term_synonyms (term_id, synonym) VALUES
(5, '纠正措施'), (5, '预防措施'), (5, '整改措施');

INSERT INTO term_hierarchy (term_id, parent_term, hierarchy_type) VALUES
(5, '质量保证', 'hypernym');

-- Equipment
INSERT INTO gmp_terms (term_id, canonical_term, term_type, synonyms, hypernyms, hyponyms, gmp_chapter, entity_type) VALUES
(6, '设备', 'entity', '["机器","仪器","装置"]', '生产资源', NULL, '第三章', 'Equipment');

INSERT INTO term_synonyms (term_id, synonym) VALUES
(6, '机器'), (6, '仪器'), (6, '装置');

INSERT INTO term_hierarchy (term_id, parent_term, hierarchy_type) VALUES
(6, '生产资源', 'hypernym');

-- Material
INSERT INTO gmp_terms (term_id, canonical_term, term_type, synonyms, hypernyms, hyponyms, gmp_chapter, entity_type) VALUES
(7, '物料', 'entity', '["原料","辅料","包材","包装材料"]', '生产资源', NULL, '第三章', 'Material');

INSERT INTO term_synonyms (term_id, synonym) VALUES
(7, '原料'), (7, '辅料'), (7, '包材'), (7, '包装材料');

INSERT INTO term_hierarchy (term_id, parent_term, hierarchy_type) VALUES
(7, '生产资源', 'hypernym');

-- Validation
INSERT INTO gmp_terms (term_id, canonical_term, term_type, synonyms, hypernyms, hyponyms, gmp_chapter, entity_type) VALUES
(8, '验证', 'entity', '["确认","IQ","OQ","PQ","DQ"]', '质量保证', NULL, '第七章', 'Validation');

INSERT INTO term_synonyms (term_id, synonym) VALUES
(8, '确认'), (8, 'IQ'), (8, 'OQ'), (8, 'PQ'), (8, 'DQ');

INSERT INTO term_hierarchy (term_id, parent_term, hierarchy_type) VALUES
(8, '质量保证', 'hypernym');

-- SOP
INSERT INTO gmp_terms (term_id, canonical_term, term_type, synonyms, hypernyms, hyponyms, gmp_chapter, entity_type) VALUES
(9, 'SOP', 'entity', '["标准操作规程","操作规程","作业指导书"]', '文件', NULL, '第四章', 'SOP');

INSERT INTO term_synonyms (term_id, synonym) VALUES
(9, '标准操作规程'), (9, '操作规程'), (9, '作业指导书');

INSERT INTO term_hierarchy (term_id, parent_term, hierarchy_type) VALUES
(9, '文件', 'hypernym');

-- Regulation
INSERT INTO gmp_terms (term_id, canonical_term, term_type, synonyms, hypernyms, hyponyms, gmp_chapter, entity_type) VALUES
(10, '法规', 'entity', '["GMP","药典","EU GMP","GMP条款"]', '合规要求', NULL, '第一章', 'Regulation');

INSERT INTO term_synonyms (term_id, synonym) VALUES
(10, 'GMP'), (10, '药典'), (10, 'EU GMP'), (10, 'GMP条款');

INSERT INTO term_hierarchy (term_id, parent_term, hierarchy_type) VALUES
(10, '合规要求', 'hypernym');

-- Department
INSERT INTO gmp_terms (term_id, canonical_term, term_type, synonyms, hypernyms, hyponyms, gmp_chapter, entity_type) VALUES
(11, '质量部', 'entity', '["质量管理部","QA","质量保证部"]', '组织架构', NULL, '第二章', 'Department');

INSERT INTO term_synonyms (term_id, synonym) VALUES
(11, '质量管理部'), (11, 'QA'), (11, '质量保证部');

INSERT INTO term_hierarchy (term_id, parent_term, hierarchy_type) VALUES
(11, '组织架构', 'hypernym');

INSERT INTO gmp_terms (term_id, canonical_term, term_type, synonyms, hypernyms, hyponyms, gmp_chapter, entity_type) VALUES
(12, '生产部', 'entity', '["生产管理部","生产车间","生产部门"]', '组织架构', NULL, '第二章', 'Department');

INSERT INTO term_synonyms (term_id, synonym) VALUES
(12, '生产管理部'), (12, '生产车间'), (12, '生产部门');

INSERT INTO term_hierarchy (term_id, parent_term, hierarchy_type) VALUES
(12, '组织架构', 'hypernym');

INSERT INTO gmp_terms (term_id, canonical_term, term_type, synonyms, hypernyms, hyponyms, gmp_chapter, entity_type) VALUES
(13, '工程部', 'entity', '["设备工程部","工程部门"]', '组织架构', NULL, '第二章', 'Department');

INSERT INTO term_synonyms (term_id, synonym) VALUES
(13, '设备工程部'), (13, '工程部门');

INSERT INTO term_hierarchy (term_id, parent_term, hierarchy_type) VALUES
(13, '组织架构', 'hypernym');

-- Audit Finding
INSERT INTO gmp_terms (term_id, canonical_term, term_type, synonyms, hypernyms, hyponyms, gmp_chapter, entity_type) VALUES
(14, '审计发现', 'entity', '["检查发现","内审发现","外审发现"]', '质量保证', NULL, '第八章', 'AuditFinding');

INSERT INTO term_synonyms (term_id, synonym) VALUES
(14, '检查发现'), (14, '内审发现'), (14, '外审发现');

INSERT INTO term_hierarchy (term_id, parent_term, hierarchy_type) VALUES
(14, '质量保证', 'hypernym');

-- Change Control
INSERT INTO gmp_terms (term_id, canonical_term, term_type, synonyms, hypernyms, hyponyms, gmp_chapter, entity_type) VALUES
(15, '变更控制', 'entity', '["变更申请","变更管理","变更评估"]', '质量保证', NULL, '第七章', 'ChangeControl');

INSERT INTO term_synonyms (term_id, synonym) VALUES
(15, '变更申请'), (15, '变更管理'), (15, '变更评估');

INSERT INTO term_hierarchy (term_id, parent_term, hierarchy_type) VALUES
(15, '质量保证', 'hypernym');

-- ============================================================
-- Section 2: GMP Term Patterns (from GMP_TERM_PATTERNS)
-- ============================================================

INSERT INTO gmp_term_patterns (term, category) VALUES
('偏差', 'entity'), ('CAPA', 'entity'), ('验证', 'entity'), ('确认', 'entity'),
('偏差调查', 'entity'), ('变更控制', 'entity'), ('设备', 'entity'),
('物料', 'entity'), ('文件', 'entity'), ('SOP', 'entity'),
('工艺', 'process'), ('生产', 'process'), ('质量', 'quality'),
('风险', 'quality'), ('审计', 'quality'),
('清洁', 'maintenance'), ('校准', 'maintenance'), ('维护', 'maintenance'),
('培训', 'personnel'), ('记录', 'document'), ('报告', 'document'),
('规程', 'document'), ('标准', 'document'),
('GMP', 'regulation'), ('药典', 'regulation'), ('法规', 'regulation'),
('无菌', 'quality'), ('厂房', 'facility'), ('空调', 'facility'),
('HVAC', 'facility'), ('压差', 'facility');

-- ============================================================
-- Section 3: Extraction Rules (from ENTITY_PATTERNS)
-- ============================================================

-- Deviation patterns
INSERT INTO extraction_rules (entity_type, pattern, canonical_form, priority) VALUES
('Deviation', '重大偏差', '重大偏差', 10),
('Deviation', '一般偏差', '一般偏差', 10),
('Deviation', '微小偏差', '微小偏差', 10),
('Deviation', 'OOS|超标|偏离', 'OOS', 8),
('Deviation', '偏差报告', '偏差报告', 7),
('Deviation', '\\w+偏差', NULL, 5);

-- CAPA patterns
INSERT INTO extraction_rules (entity_type, pattern, canonical_form, priority) VALUES
('CAPA', 'CAPA[-_]?\\d+', 'CAPA', 10),
('CAPA', '纠正措施', 'CAPA', 8),
('CAPA', '预防措施', 'CAPA', 8),
('CAPA', 'CAPA', 'CAPA', 7);

-- Equipment patterns
INSERT INTO extraction_rules (entity_type, pattern, canonical_form, priority) VALUES
('Equipment', '\\w+机[器]?(?!动)', NULL, 7),
('Equipment', '\\w+设备', NULL, 7),
('Equipment', '混合机|压片机|烘干箱|灭菌柜|灌装机', NULL, 10);

-- Material patterns
INSERT INTO extraction_rules (entity_type, pattern, canonical_form, priority) VALUES
('Material', '原料[药]?', '原料', 10),
('Material', '辅料', '辅料', 10),
('Material', '中间体', '中间体', 10),
('Material', '成品', '成品', 10),
('Material', '包材|包装材料', '包材', 10);

-- Validation patterns
INSERT INTO extraction_rules (entity_type, pattern, canonical_form, priority) VALUES
('Validation', 'IQ|安装确认', 'IQ', 10),
('Validation', 'OQ|运行确认', 'OQ', 10),
('Validation', 'PQ|性能确认', 'PQ', 10),
('Validation', 'DQ|设计确认', 'DQ', 10),
('Validation', '清洁验证', '清洁验证', 10),
('Validation', '工艺验证', '工艺验证', 10),
('Validation', '验证方案', '验证方案', 8);

-- Regulation patterns
INSERT INTO extraction_rules (entity_type, pattern, canonical_form, priority) VALUES
('Regulation', 'GMP第\\d+条', 'GMP条款', 10),
('Regulation', 'GMP第[一二三四五六七八九十\\d]+条', 'GMP条款', 10),
('Regulation', '药典\\d{4}版', '药典', 10),
('Regulation', 'EU\\s*GMP|Annex\\s*\\d+', 'EU GMP', 10);

-- Department patterns
INSERT INTO extraction_rules (entity_type, pattern, canonical_form, priority) VALUES
('Department', '质量授权人|QA', '质量授权人', 10),
('Department', '生产负责人', '生产负责人', 10),
('Department', '质量负责人', '质量负责人', 10),
('Department', '工程负责人', '工程负责人', 10),
('Department', '质量部', '质量部', 10),
('Department', '生产部', '生产部', 10),
('Department', '工程部', '工程部', 10),
('Department', '仓储部', '仓储部', 10),
('Department', '采购部', '采购部', 10);

-- SOP patterns
INSERT INTO extraction_rules (entity_type, pattern, canonical_form, priority) VALUES
('SOP', '\\w+标准操作规程', 'SOP', 10),
('SOP', '[A-Z]{2,}[-_]\\d+', 'SOP', 8);
