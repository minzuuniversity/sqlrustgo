#!/usr/bin/env python3
"""
Layer 0 Knowledge Extraction Pipeline for GMP Platform

This script processes all GMP documents through the 5-step Layer 0 pipeline:
  Step 0: Load GMP terms from gmp_terms.sql
  Step 1: NER - Extract named entities → doc_entities
  Step 2: RE  - Extract relations → doc_relations
  Step 3: CLF - Classify documents → doc_taxonomy
  Step 4: SCR - Score relevance → doc_relevance
  Step 5: Write to Layer 2 graph (stub)

Usage:
  python3 scripts/run_layer0_pipeline.py [--db data/gmp.db] [--limit N]
"""

import sqlite3
import re
import json
import sys
import argparse
import time
from pathlib import Path

# ============ GMP Terms Dictionary ============
# Loaded from gmp_terms table (already in DB)

# ============ NER Patterns (from GMP domain knowledge) ============
NER_PATTERNS = {
    # SOP/规程
    'SOP': [
        (r'SOP[-_]?\w+-\d+', 'SOP'),
        (r'\w+操作规程', 'SOP'),
        (r'\w+管理规程', 'SOP'),
        (r'\w+标准操作规程', 'SOP'),
        (r'[A-Z]{2,}[-_]\d+', 'SOP'),
    ],
    # 偏差类型
    'Deviation': [
        (r'重大偏差', '重大偏差'),
        (r'一般偏差', '一般偏差'),
        (r'微小偏差', '微小偏差'),
        (r'OOS|超标|偏离', 'OOS'),
        (r'偏差报告', '偏差报告'),
    ],
    # CAPA
    'CAPA': [
        (r'CAPA[-_]?\d+', 'CAPA'),
        (r'纠正措施', 'CAPA'),
        (r'预防措施', 'CAPA'),
        (r'CAPA', 'CAPA'),
    ],
    # 设备
    'Equipment': [
        (r'\w+机[器]?(?!动)', 'Equipment'),
        (r'\w+设备', 'Equipment'),
        (r'混合机|压片机|烘干箱|灭菌柜|灌装机', 'Equipment'),
    ],
    # 物料
    'Material': [
        (r'原料[药]?', '原料'),
        (r'辅料', '辅料'),
        (r'中间体', '中间体'),
        (r'成品', '成品'),
        (r'包材|包装材料', '包材'),
    ],
    # 验证类型
    'Validation': [
        (r'IQ|安装确认', 'IQ'),
        (r'OQ|运行确认', 'OQ'),
        (r'PQ|性能确认', 'PQ'),
        (r'DQ|设计确认', 'DQ'),
        (r'清洁验证', '清洁验证'),
        (r'工艺验证', '工艺验证'),
        (r'验证方案', '验证方案'),
    ],
    # 法规条款
    'Regulation': [
        (r'GMP第\d+条', 'GMP条款'),
        (r'GMP第[一二三四五六七八九十\d]+条', 'GMP条款'),
        (r'药典\d{4}版', '药典'),
        (r'EU\s*GMP|Annex\s*\d+', 'EU GMP'),
    ],
    # 人员角色
    'Operator': [
        (r'质量授权人|QA', '质量授权人'),
        (r'生产负责人', '生产负责人'),
        (r'质量负责人', '质量负责人'),
        (r'工程负责人', '工程负责人'),
    ],
    # 部门
    'Department': [
        (r'质量部', '质量部'),
        (r'生产部', '生产部'),
        (r'工程部', '工程部'),
        (r'仓储部', '仓储部'),
        (r'采购部', '采购部'),
    ],
}

# ============ Relation Templates (from gmp-re crate) ============
RELATION_TEMPLATES = [
    # (pattern, relation_type, src_type_hint, dst_type_hint)
    (r'(\w{2,20})触发([A-Z]{2,}-?\d+)', 'TRIGGERS', 'Deviation', 'CAPA'),
    (r'(\w{2,20})引发([A-Z]{2,}-?\d+)', 'TRIGGERS', 'Deviation', 'CAPA'),
    (r'(\w{2,20})导致([A-Z]{2,}-?\d+)', 'TRIGGERS', 'Deviation', 'CAPA'),
    (r'审计发现[的]?(\w{2,10})启动([A-Z]{2,}-?\d+)', 'GENERATES', 'AuditFinding', 'CAPA'),
    (r'(\w{2,10})需采取([A-Z]{2,}-?\d+)', 'GENERATES', 'AuditFinding', 'CAPA'),
    (r'(\w{2,10})需进行(IQ|OQ|PQ|DQ|验证|确认)', 'REQUIRES', 'Equipment', 'Validation'),
    (r'(\w{2,10})设备需要(IQ|OQ|PQ)', 'REQUIRES', 'Equipment', 'Validation'),
    (r'(GMP第?\d+条)[规]?定[了]?(\w{2,10})', 'MANDATES', 'Regulation', 'SOP'),
    (r'(\w{2,10})由(质量部|生产部|工程部|仓储部)管理', 'MANAGED_BY', 'SOP', 'Department'),
    (r'(\w{2,10})属于(第[一二三四五六七八九十\d]+章)', 'BELONGS_TO', 'SOP', 'Chapter'),
]

# ============ Taxonomy Rules (from gmp-classifier crate) ============
TAXONOMY_TOPIC_PATTERNS = [
    # topic
    (r'偏差|偏离|异常', 'topic', '偏差', 10),
    (r'变更控制|变更申请|变更管理', 'topic', '变更控制', 10),
    (r'验证方案|验证报告|确认方案|IQ|OQ|PQ', 'topic', '验证', 10),
    (r'设备管理|设备维护|设备校准', 'topic', '设备', 10),
    (r'物料管理|物料控制|仓储管理', 'topic', '物料', 10),
    (r'文件管理|文档管理|SOP管理', 'topic', '文件', 10),
    (r'人员培训|培训管理|资质确认', 'topic', '人员', 10),
    (r'厂房设施|洁净区|环境监控|HVAC', 'topic', '厂房', 10),
]

TAXONOMY_DOCTYPE_PATTERNS = [
    (r'SOP[-_]?\d+|标准操作规程', 'doc_type', 'SOP', 20),
    (r'管理规程|管理文件', 'doc_type', '管理规程', 15),
    (r'批生产记录|批记录|检验记录|培训记录', 'doc_type', '记录', 15),
    (r'验证报告|确认报告|调查报告|审计报告', 'doc_type', '报告', 15),
    (r'验证方案|确认方案|培训计划|审计计划', 'doc_type', '方案', 15),
    (r'GMP[第]?\d*[条章]?|药典', 'doc_type', '法规', 10),
]

TAXONOMY_RISK_PATTERNS = [
    (r'重大偏差|关键设备|关键工艺参数|无菌|高风险', 'risk_level', '高', 20),
    (r'一般偏差|重要|中等', 'risk_level', '中', 15),
    (r'微小偏差|低风险|次要', 'risk_level', '低', 15),
]

TAXONOMY_DEPT_PATTERNS = [
    (r'质量部|质量管理部|QA', 'department', '质量部', 20),
    (r'生产部|生产管理部|生产车间', 'department', '生产部', 20),
    (r'工程部|设备工程部', 'department', '工程部', 20),
    (r'仓储部|仓库', 'department', '仓储部', 20),
    (r'采购部', 'department', '采购部', 20),
]

# ============ Scorer Rules ============
GMP_TERM_PATTERNS = [
    '偏差', 'CAPA', '验证', '确认', '偏差调查', '变更控制', '设备',
    '物料', '文件', 'SOP', '工艺', '生产', '质量', '风险', '审计',
    '清洁', '校准', '维护', '培训', '记录', '报告', '规程', '标准',
    'GMP', '药典', '法规', '无菌', '厂房', '空调', 'HVAC', '压差',
]

PROCESS_INDICATORS = {
    'has_procedure': [r'程序', r'流程', r'步骤', r'操作', r'规定'],
    'has_responsibility': [r'职责', r'责任人', r'负责', r'部门'],
    'has_acceptance_criteria': [r'标准', r'要求', r'判定', r'合格'],
    'has_records': [r'记录', r'表单', r'填写', r'签名'],
    'has_attachment': [r'附件', r'附录', r'附表'],
}

REG_PATTERNS = [
    (r'GMP第(\d+)条', 'gmp_articles'),
    (r'GMP第[一二三四五六七八九十]+条', 'gmp_articles'),
    (r'第[一二三四五六七八九十]+章', 'chapter_refs'),
    (r'药典', 'other_regs'),
    (r'EU\s*GMP|Annex', 'other_regs'),
]


def create_layer0_tables(conn):
    """Create Layer 0 tables if they don't exist."""
    cursor = conn.cursor()
    
    # Create gmp_terms with the same schema as the SQL file
    cursor.execute('''
        CREATE TABLE IF NOT EXISTS gmp_terms (
            term_id INTEGER PRIMARY KEY,
            canonical_term TEXT NOT NULL,
            term_type TEXT NOT NULL,
            hypernym TEXT,
            gmp_chapter TEXT,
            created_at INTEGER
        )
    ''')
    
    # gmp_synonyms table (from SQL file)
    cursor.execute('''
        CREATE TABLE IF NOT EXISTS gmp_synonyms (
            synonym_id INTEGER PRIMARY KEY,
            term_id INTEGER NOT NULL,
            synonym TEXT NOT NULL,
            created_at INTEGER
        )
    ''')
    
    cursor.execute('''
        CREATE TABLE IF NOT EXISTS doc_entities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            doc_id TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            surface_form TEXT NOT NULL,
            canonical_term TEXT NOT NULL,
            start_pos INTEGER,
            end_pos INTEGER,
            confidence REAL DEFAULT 1.0,
            created_at INTEGER DEFAULT (strftime('%s', 'now'))
        )
    ''')
    
    cursor.execute('''
        CREATE TABLE IF NOT EXISTS doc_relations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            doc_id TEXT NOT NULL,
            relation_type TEXT NOT NULL,
            src_entity TEXT NOT NULL,
            dst_entity TEXT NOT NULL,
            confidence REAL DEFAULT 1.0,
            created_at INTEGER DEFAULT (strftime('%s', 'now'))
        )
    ''')
    
    cursor.execute('''
        CREATE TABLE IF NOT EXISTS doc_taxonomy (
            doc_id TEXT PRIMARY KEY,
            topic TEXT,
            sub_topic TEXT,
            doc_type TEXT,
            regulation TEXT,
            risk_level TEXT,
            department TEXT,
            confidence REAL DEFAULT 0.0,
            created_at INTEGER DEFAULT (strftime('%s', 'now'))
        )
    ''')
    
    cursor.execute('''
        CREATE TABLE IF NOT EXISTS doc_relevance (
            doc_id TEXT PRIMARY KEY,
            overall_score REAL DEFAULT 0.0,
            term_density REAL,
            reg_citation REAL,
            process_completeness REAL,
            entity_density REAL,
            dept_coverage REAL,
            confidence REAL DEFAULT 0.0,
            created_at INTEGER DEFAULT (strftime('%s', 'now'))
        )
    ''')
    
    # Create indexes
    cursor.execute('CREATE INDEX IF NOT EXISTS idx_doc_entities_doc_id ON doc_entities(doc_id)')
    cursor.execute('CREATE INDEX IF NOT EXISTS idx_doc_relations_doc_id ON doc_relations(doc_id)')
    cursor.execute('CREATE INDEX IF NOT EXISTS idx_doc_entities_type ON doc_entities(entity_type)')
    cursor.execute('CREATE INDEX IF NOT EXISTS idx_doc_entities_canonical ON doc_entities(canonical_term)')
    
    conn.commit()
    print("[Step 0] Created Layer 0 tables: gmp_terms, gmp_synonyms, doc_entities, doc_relations, doc_taxonomy, doc_relevance")


def load_gmp_terms(conn, terms_file):
    """Load GMP terms from SQL file into gmp_terms and gmp_synonyms tables."""
    if not Path(terms_file).exists():
        print(f"[Step 0] Terms file not found: {terms_file}, skipping...")
        return 0
    
    cursor = conn.cursor()
    
    # Check if already loaded
    cursor.execute('SELECT COUNT(*) FROM gmp_terms')
    result = cursor.fetchone()
    term_count = result[0] if result else 0
    if term_count > 0:
        print(f"[Step 0] GMP terms already loaded ({term_count} terms), skipping...")
        return term_count
    
    # Read SQL file
    with open(terms_file, 'r', encoding='utf-8') as f:
        sql_content = f.read()
    
    # Execute the SQL file directly (it contains DROP, CREATE, INSERT statements)
    cursor.executescript(sql_content)
    conn.commit()
    
    # Count loaded terms
    cursor.execute('SELECT COUNT(*) FROM gmp_terms')
    term_count = cursor.fetchone()[0]
    cursor.execute('SELECT COUNT(*) FROM gmp_synonyms')
    syn_count = cursor.fetchone()[0]
    
    print(f"[Step 0] Loaded {term_count} GMP terms and {syn_count} synonyms from {terms_file}")
    return term_count


def get_gmp_terms_map(conn):
    """Build a map of GMP terms for NER."""
    cursor = conn.cursor()
    
    # Build map from gmp_terms
    terms_map = {}
    cursor.execute('SELECT term_id, canonical_term, term_type FROM gmp_terms')
    for row in cursor.fetchall():
        term_id, canonical, term_type = row
        terms_map[canonical.strip()] = {'type': term_type, 'term_id': term_id}
    
    # Add synonyms
    cursor.execute('SELECT term_id, synonym FROM gmp_synonyms')
    syn_map = {}
    for row in cursor.fetchall():
        term_id, synonym = row
        syn_map[term_id] = synonym.strip()
    
    # Update terms_map with synonyms
    for term, info in terms_map.items():
        if info['term_id'] in syn_map:
            info['synonym'] = syn_map[info['term_id']]
    
    return terms_map


def extract_entities(text, terms_map):
    """Extract GMP entities from text using dictionary + patterns."""
    entities = []
    seen = set()
    
    # Method 1: Dictionary lookup from GMP terms
    for term, info in terms_map.items():
        if len(term) >= 2:
            pattern = re.escape(term)
            for match in re.finditer(pattern, text):
                key = (match.start(), match.end(), term)
                if key not in seen:
                    seen.add(key)
                    entities.append({
                        'surface_form': match.group(),
                        'canonical_term': info.get('canonical', term),
                        'entity_type': info.get('type', 'Unknown'),
                        'start_pos': match.start(),
                        'end_pos': match.end(),
                        'confidence': 1.0
                    })
    
    # Method 2: Regex patterns
    for entity_type, patterns in NER_PATTERNS.items():
        for pattern, canonical in patterns:
            for match in re.finditer(pattern, text):
                key = (match.start(), match.end(), canonical)
                if key not in seen:
                    seen.add(key)
                    entities.append({
                        'surface_form': match.group(),
                        'canonical_term': canonical,
                        'entity_type': entity_type,
                        'start_pos': match.start(),
                        'end_pos': match.end(),
                        'confidence': 0.9
                    })
    
    return entities


def extract_relations(text, entities):
    """Extract relations from text using templates."""
    relations = []
    
    for pattern, rel_type, src_hint, dst_hint in RELATION_TEMPLATES:
        for match in re.finditer(pattern, text):
            src = match.group(1).strip() if match.lastindex >= 1 else ''
            dst = match.group(2).strip() if match.lastindex >= 2 else ''
            
            if src and dst and len(src) >= 2 and len(dst) >= 2:
                relations.append({
                    'relation_type': rel_type,
                    'src_entity': src,
                    'dst_entity': dst,
                    'confidence': 0.85
                })
    
    return relations


def classify_document(title, content):
    """Classify document using taxonomy rules."""
    text = f"{title} {content[:2000]}"  # Limit content length
    
    result = {
        'topic': None,
        'sub_topic': None,
        'doc_type': None,
        'risk_level': None,
        'department': None,
        'regulation': None,
        'confidence': 0.0
    }
    
    matched_dimensions = 0
    
    # Topic
    best_topic = None
    best_topic_priority = 0
    for pattern, dim, value, priority in TAXONOMY_TOPIC_PATTERNS:
        if re.search(pattern, text) and priority > best_topic_priority:
            best_topic_priority = priority
            best_topic = value
    if best_topic:
        result['topic'] = best_topic
        matched_dimensions += 1
    
    # Doc type
    best_doctype = None
    best_doctype_priority = 0
    for pattern, dim, value, priority in TAXONOMY_DOCTYPE_PATTERNS:
        if re.search(pattern, text) and priority > best_doctype_priority:
            best_doctype_priority = priority
            best_doctype = value
    if best_doctype:
        result['doc_type'] = best_doctype
        matched_dimensions += 1
    
    # Risk level
    best_risk = None
    best_risk_priority = 0
    for pattern, dim, value, priority in TAXONOMY_RISK_PATTERNS:
        if re.search(pattern, text) and priority > best_risk_priority:
            best_risk_priority = priority
            best_risk = value
    if best_risk:
        result['risk_level'] = best_risk
        matched_dimensions += 1
    
    # Department
    best_dept = None
    best_dept_priority = 0
    for pattern, dim, value, priority in TAXONOMY_DEPT_PATTERNS:
        if re.search(pattern, text) and priority > best_dept_priority:
            best_dept_priority = priority
            best_dept = value
    if best_dept:
        result['department'] = best_dept
        matched_dimensions += 1
    
    result['confidence'] = min(matched_dimensions * 0.25, 1.0)
    return result


def score_document(title, content, entity_count):
    """Score document relevance using scoring dimensions."""
    text = f"{title} {content}"
    content_len = len(content) if content else 0
    
    # Term density
    term_matches = sum(1 for term in GMP_TERM_PATTERNS if term in text)
    term_density = min(term_matches / 20.0, 1.0) if content_len > 0 else 0.0
    
    # Regulatory citation
    gmp_count = len(re.findall(r'GMP第[一二三四五六七八九十\d]+条|GMP第\d+条', text))
    chapter_count = len(re.findall(r'第[一二三四五六七八九十]+章|第\d+章', text))
    other_reg_count = len(re.findall(r'药典|EU\s*GMP|Annex|FDA', text))
    reg_citation = min((gmp_count * 2.0 + chapter_count * 1.5 + other_reg_count) / 10.0, 1.0)
    
    # Process completeness
    process_score = 0.0
    if re.search(r'程序|流程|步骤|操作|规定', text): process_score += 0.25
    if re.search(r'职责|责任人|负责|部门', text): process_score += 0.20
    if re.search(r'标准|要求|判定|合格', text): process_score += 0.25
    if re.search(r'记录|表单|填写|签名', text): process_score += 0.20
    if re.search(r'附件|附录|附表', text): process_score += 0.10
    process_completeness = process_score
    
    # Entity density
    entity_density = min(entity_count / 30.0, 1.0) if content_len > 0 else 0.0
    
    # Department coverage
    dept_coverage = 1.0 if re.search(r'质量部|生产部|工程部|仓储部|采购部', text) else 0.0
    
    # Overall score (weighted)
    overall = (
        term_density * 0.25 +
        reg_citation * 0.25 +
        process_completeness * 0.20 +
        entity_density * 0.15 +
        dept_coverage * 0.15
    )
    
    # Confidence
    confidence = min(content_len / 500.0, 1.0) if content_len > 0 else 0.1
    
    return {
        'overall_score': round(overall, 4),
        'term_density': round(term_density, 4),
        'reg_citation': round(reg_citation, 4),
        'process_completeness': round(process_completeness, 4),
        'entity_density': round(entity_density, 4),
        'dept_coverage': round(dept_coverage, 4),
        'confidence': round(confidence, 4)
    }


def process_documents(conn, limit=None):
    """Process all documents through Layer 0 pipeline."""
    cursor = conn.cursor()
    
    # Get documents with their chunks content
    cursor.execute('''
        SELECT d.doc_id, d.title, d.doc_type, d.department, GROUP_CONCAT(c.content, ' ') as content
        FROM gmp_documents d
        LEFT JOIN gmp_document_chunks c ON d.doc_id = c.doc_id
        GROUP BY d.doc_id
    ''')
    
    documents = cursor.fetchall()
    total = len(documents)
    print(f"\n[Pipeline] Processing {total} documents...")
    
    # Get GMP terms map
    terms_map = get_gmp_terms_map(conn)
    print(f"[Pipeline] Loaded {len(terms_map)} GMP terms for NER")
    
    # Clear existing Layer 0 data
    cursor.execute('DELETE FROM doc_entities')
    cursor.execute('DELETE FROM doc_relations')
    cursor.execute('DELETE FROM doc_taxonomy')
    cursor.execute('DELETE FROM doc_relevance')
    conn.commit()
    
    entity_total = 0
    relation_total = 0
    taxonomy_total = 0
    relevance_total = 0
    
    start_time = time.time()
    
    for i, (doc_id, title, doc_type, department, content) in enumerate(documents):
        if content is None:
            content = ''
        if title is None:
            title = ''
        
        title_content = f"{title} {content[:5000]}"
        
        # Step 1: NER - Extract entities
        entities = extract_entities(title_content, terms_map)
        for ent in entities:
            cursor.execute('''
                INSERT INTO doc_entities (doc_id, entity_type, surface_form, canonical_term, start_pos, end_pos, confidence)
                VALUES (?, ?, ?, ?, ?, ?, ?)
            ''', (doc_id, ent['entity_type'], ent['surface_form'], ent['canonical_term'],
                  ent['start_pos'], ent['end_pos'], ent['confidence']))
            entity_total += 1
        
        # Step 2: RE - Extract relations
        relations = extract_relations(title_content, entities)
        for rel in relations:
            cursor.execute('''
                INSERT INTO doc_relations (doc_id, relation_type, src_entity, dst_entity, confidence)
                VALUES (?, ?, ?, ?, ?)
            ''', (doc_id, rel['relation_type'], rel['src_entity'], rel['dst_entity'], rel['confidence']))
            relation_total += 1
        
        # Step 3: Classification (use DB metadata + content analysis)
        taxonomy = classify_document(title, content)
        # Override with DB metadata if available
        if department and not taxonomy['department']:
            taxonomy['department'] = department
        if doc_type and not taxonomy['doc_type']:
            taxonomy['doc_type'] = doc_type
        cursor.execute('''
            INSERT OR REPLACE INTO doc_taxonomy (doc_id, topic, sub_topic, doc_type, risk_level, department, confidence)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        ''', (doc_id, taxonomy['topic'], taxonomy['sub_topic'], taxonomy['doc_type'],
              taxonomy['risk_level'], taxonomy['department'], taxonomy['confidence']))
        if taxonomy['confidence'] > 0:
            taxonomy_total += 1
        
        # Step 4: Relevance scoring
        entity_count = len(entities)
        scores = score_document(title, content, entity_count)
        cursor.execute('''
            INSERT OR REPLACE INTO doc_relevance (doc_id, overall_score, term_density, reg_citation, process_completeness, entity_density, dept_coverage, confidence)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ''', (doc_id, scores['overall_score'], scores['term_density'], scores['reg_citation'],
              scores['process_completeness'], scores['entity_density'], scores['dept_coverage'], scores['confidence']))
        relevance_total += 1
        
        if (i + 1) % 200 == 0:
            conn.commit()
            elapsed = time.time() - start_time
            rate = (i + 1) / elapsed
            print(f"  Processed {i+1}/{total} ({rate:.1f} docs/sec)")
    
    conn.commit()
    elapsed = time.time() - start_time
    
    print(f"\n[Pipeline] Complete! Processed {total} documents in {elapsed:.1f}s ({total/elapsed:.1f} docs/sec)")
    print(f"  doc_entities:   {entity_total} entities inserted")
    print(f"  doc_relations:  {relation_total} relations inserted")
    print(f"  doc_taxonomy:   {taxonomy_total} classified (confidence > 0)")
    print(f"  doc_relevance:  {relevance_total} scored")
    
    return {
        'entities': entity_total,
        'relations': relation_total,
        'taxonomy': taxonomy_total,
        'relevance': relevance_total
    }


def show_stats(conn):
    """Show Layer 0 statistics."""
    cursor = conn.cursor()
    
    print("\n" + "=" * 50)
    print("Layer 0 Statistics")
    print("=" * 50)
    
    tables = ['gmp_terms', 'doc_entities', 'doc_relations', 'doc_taxonomy', 'doc_relevance']
    for table in tables:
        cursor.execute(f'SELECT COUNT(*) FROM {table}')
        count = cursor.fetchone()[0]
        print(f"  {table:20s}: {count:,} rows")
    
    # Entity type distribution
    print("\n  Entity Type Distribution:")
    cursor.execute('''
        SELECT entity_type, COUNT(*) as cnt 
        FROM doc_entities 
        GROUP BY entity_type 
        ORDER BY cnt DESC 
        LIMIT 10
    ''')
    for row in cursor.fetchall():
        print(f"    {row[0]:20s}: {row[1]:,}")
    
    # Relation type distribution
    print("\n  Relation Type Distribution:")
    cursor.execute('''
        SELECT relation_type, COUNT(*) as cnt 
        FROM doc_relations 
        GROUP BY relation_type 
        ORDER BY cnt DESC
    ''')
    for row in cursor.fetchall():
        print(f"    {row[0]:20s}: {row[1]:,}")
    
    # Taxonomy topic distribution
    print("\n  Taxonomy Topic Distribution:")
    cursor.execute('''
        SELECT topic, COUNT(*) as cnt 
        FROM doc_taxonomy 
        WHERE topic IS NOT NULL 
        GROUP BY topic 
        ORDER BY cnt DESC
    ''')
    for row in cursor.fetchall():
        print(f"    {row[0]:20s}: {row[1]:,}")
    
    # Relevance score distribution
    print("\n  Relevance Score Distribution:")
    cursor.execute('''
        SELECT 
            CASE 
                WHEN overall_score >= 0.7 THEN 'high (>=0.7)'
                WHEN overall_score >= 0.4 THEN 'medium (0.4-0.7)'
                ELSE 'low (<0.4)'
            END as bucket,
            COUNT(*) as cnt
        FROM doc_relevance
        GROUP BY bucket
        ORDER BY bucket
    ''')
    for row in cursor.fetchall():
        print(f"    {row[0]:20s}: {row[1]:,}")
    
    # Average scores
    cursor.execute('SELECT AVG(overall_score), AVG(term_density), AVG(reg_citation) FROM doc_relevance')
    row = cursor.fetchone()
    print(f"\n  Avg Overall Score: {row[0]:.4f}" if row[0] else "")
    print(f"  Avg Term Density:  {row[1]:.4f}" if row[1] else "")
    print(f"  Avg Reg Citation:   {row[2]:.4f}" if row[2] else "")
    
    print("=" * 50)


def main():
    parser = argparse.ArgumentParser(description='Run Layer 0 Knowledge Extraction Pipeline')
    parser.add_argument('--db', default='data/gmp.db', help='Path to SQLite database')
    parser.add_argument('--terms', default='docs/releases/v1.1.0/gmp_terms.sql', help='Path to GMP terms SQL file')
    parser.add_argument('--limit', type=int, default=None, help='Limit number of documents to process')
    parser.add_argument('--stats', action='store_true', help='Show stats and exit')
    args = parser.parse_args()
    
    # Resolve paths relative to project root
    db_path = Path(args.db)
    if not db_path.is_absolute():
        db_path = Path(__file__).parent.parent / db_path
    
    terms_path = Path(args.terms)
    if not terms_path.is_absolute():
        terms_path = Path(__file__).parent.parent / terms_path
    
    if not db_path.exists():
        print(f"Error: Database not found at {db_path}")
        sys.exit(1)
    
    print(f"Opening database: {db_path}")
    conn = sqlite3.connect(str(db_path))
    
    # Create tables
    create_layer0_tables(conn)
    
    # Load terms
    if terms_path.exists():
        load_gmp_terms(conn, str(terms_path))
    
    if args.stats:
        show_stats(conn)
    else:
        # Run pipeline
        stats = process_documents(conn, limit=args.limit)
        show_stats(conn)
    
    conn.close()
    print("\nDone!")


if __name__ == '__main__':
    main()
