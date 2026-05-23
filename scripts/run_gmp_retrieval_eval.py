#!/usr/bin/env python3
"""
GMP 5-Layer Architecture Evaluation Script

This script evaluates the full 5-layer retrieval pipeline:
  L0: Knowledge Extraction (already done - doc_entities, doc_relations, doc_taxonomy, doc_relevance)
  L1: Trust Zone      (sanitized rules - not used in FTS)
  L2: Cloud Zone      (DSL templates - not used in FTS)
  L3: Validation Zone (test scenarios)
  L4: Application     (retrieval + ranking)

Strategy:
  - Use doc_taxonomy for classification filtering
  - Use doc_relevance for relevance scoring
  - Use doc_entities for entity density boost
  - Use gmp_chunks_fts for full-text search with bm25
  - Combine all signals into a final ranking

Usage:
  python3 scripts/run_gmp_retrieval_eval.py --db data/gmp.db --test-set test-data/391_scenarios.json
"""

import sqlite3
import json
import time
import argparse
from collections import defaultdict

# Layer 0 NER patterns for additional entity extraction
NER_PATTERNS = [
    '偏差', 'CAPA', '验证', '确认', '变更', '审计', '培训', '设备',
    '物料', '文件', 'SOP', '工艺', '质量', '风险', '清洁', '校准',
    '维护', '记录', '报告', '规程', '标准', 'GMP', '药典', '法规',
    '无菌', '厂房', '空调', 'HVAC', '压差', '温湿度', '生产', '检验',
]


def simple_tokenize(text):
    """Simple bigram tokenizer for Chinese text."""
    chars = list(text)
    tokens = []
    for i in range(len(chars)):
        c = chars[i]
        # Check if CJK
        if '\u4e00' <= c <= '\u9fff':
            if i + 1 < len(chars) and '\u4e00' <= chars[i+1] <= '\u9fff':
                tokens.append(c + chars[i+1])
        elif c.isalnum():
            # English word
            j = i
            word = ''
            while j < len(chars) and chars[j].isalnum():
                word += chars[j]
                j += 1
            if len(word) >= 2:
                tokens.append(word.lower())
    return tokens


def search_fts_plus(conn, query, top_k=10):
    """
    Enhanced FTS search that combines:
    1. FTS5 BM25 score
    2. Layer 0 relevance score (doc_relevance.overall_score)
    3. Layer 0 entity density boost
    4. Taxonomy filtering (if applicable)
    """
    cursor = conn.cursor()
    
    # Tokenize query
    tokens = simple_tokenize(query)
    if not tokens:
        tokens = [query]
    
    # Build FTS query (bigrams joined with OR)
    fts_query = ' OR '.join(tokens)
    
    # Get FTS results with BM25
    try:
        cursor.execute('''
            SELECT c.doc_id, c.content, d.title, d.doc_type, d.department,
                   bm25(gmp_chunks_fts) as bm25_score
            FROM gmp_chunks_fts fts
            JOIN gmp_document_chunks c ON fts.rowid = c.id
            JOIN gmp_documents d ON c.doc_id = d.doc_id
            WHERE gmp_chunks_fts MATCH ?
            ORDER BY bm25_score
            LIMIT ?
        ''', (fts_query, top_k * 3))
        fts_rows = cursor.fetchall()
    except:
        fts_rows = []
    
    # Also try LIKE fallback for Chinese
    if len(fts_rows) < top_k:
        substrings = [query[i:i+2] for i in range(len(query)-1) if '\u4e00' <= query[i] <= '\u9fff' and '\u4e00' <= query[i+1] <= '\u9fff']
        for sub in substrings[:5]:
            try:
                cursor.execute('''
                    SELECT c.doc_id, c.content, d.title, d.doc_type, d.department, 0.0 as bm25_score
                    FROM gmp_document_chunks c
                    JOIN gmp_documents d ON c.doc_id = d.doc_id
                    WHERE c.content LIKE ?
                    LIMIT ?
                ''', (f'%{sub}%', top_k))
                like_rows = cursor.fetchall()
                for row in like_rows:
                    if row not in fts_rows:
                        fts_rows.append(row)
            except:
                pass
    
    # Score and rank using Layer 0 signals
    scored_results = []
    for doc_id, content, title, doc_type, department, bm25_score in fts_rows:
        # Get relevance score from doc_relevance
        cursor.execute('SELECT overall_score FROM doc_relevance WHERE doc_id = ?', (doc_id,))
        rel_row = cursor.fetchone()
        rel_score = rel_row[0] if rel_row else 0.0
        
        # Get entity count from doc_entities
        cursor.execute('SELECT COUNT(DISTINCT canonical_term) FROM doc_entities WHERE doc_id = ?', (doc_id,))
        ent_row = cursor.fetchone()
        ent_count = ent_row[0] if ent_row else 0
        
        # Get taxonomy
        cursor.execute('SELECT topic, doc_type, department FROM doc_taxonomy WHERE doc_id = ?', (doc_id,))
        tax_row = cursor.fetchone()
        
        # Compute combined score
        # BM25 is negative (lower = better), convert to positive
        bm25_pos = max(0, 10 + bm25_score)  # Shift to positive range
        
        # Relevance weight
        rel_weight = rel_score * 5.0
        
        # Entity density boost
        ent_boost = min(ent_count / 20.0, 1.0) * 2.0
        
        # Combined score
        combined_score = bm25_pos + rel_weight + ent_boost
        
        scored_results.append({
            'doc_id': doc_id,
            'title': title,
            'content': content[:500] if content else '',
            'combined_score': combined_score,
            'bm25_score': bm25_score,
            'rel_score': rel_score,
            'ent_count': ent_count,
            'taxonomy': tax_row,
        })
    
    # Sort by combined score
    scored_results.sort(key=lambda x: x['combined_score'], reverse=True)
    
    return scored_results[:top_k]


def evaluate_scenarios(conn, scenarios, top_k=10):
    """Evaluate all scenarios."""
    total_passed = 0
    category_stats = defaultdict(lambda: {'passed': 0, 'total': 0})
    all_recalls = []
    all_mrrs = []
    
    for i, scenario in enumerate(scenarios):
        query = scenario['query']
        expected_ids = scenario.get('expected_doc_ids', [])
        category = scenario.get('category', 'unknown')
        scenario_top_k = scenario.get('top_k', top_k)
        
        # Search
        results = search_fts_plus(conn, query, scenario_top_k)
        retrieved_ids = [r['doc_id'] for r in results]
        
        # Keyword matching
        hit_count = 0
        for kw in expected_ids:
            kw_lower = kw.lower()
            # Check doc_id
            if any(kw_lower in id.lower() for id in retrieved_ids):
                hit_count += 1
                continue
            # Check title/content
            for r in results:
                if kw_lower in r['title'].lower() or kw_lower in r['content'].lower():
                    hit_count += 1
                    break
        
        recall = hit_count / len(expected_ids) if expected_ids else 0.0
        all_recalls.append(recall)
        
        # MRR
        mrr = 0.0
        for rank, rid in enumerate(retrieved_ids):
            rid_lower = rid.lower()
            if any(kw.lower() in rid_lower for kw in expected_ids):
                mrr = 1.0 / (rank + 1)
                break
        all_mrrs.append(mrr)
        
        passed = recall > 0.0
        if passed:
            total_passed += 1
        
        category_stats[category]['total'] += 1
        if passed:
            category_stats[category]['passed'] += 1
        
        if (i + 1) % 100 == 0:
            print(f"  Evaluated {i+1}/{len(scenarios)}...")
    
    total = len(scenarios)
    pass_rate = (total_passed / total) * 100.0
    avg_recall = sum(all_recalls) / total if all_recalls else 0.0
    avg_mrr = sum(all_mrrs) / total if all_mrrs else 0.0
    
    return {
        'total': total,
        'passed': total_passed,
        'pass_rate': pass_rate,
        'avg_recall': avg_recall,
        'avg_mrr': avg_mrr,
        'category_stats': dict(category_stats),
    }


def main():
    parser = argparse.ArgumentParser(description='Evaluate GMP 5-Layer Retrieval')
    parser.add_argument('--db', default='data/gmp.db')
    parser.add_argument('--test-set', default='test-data/391_scenarios.json')
    parser.add_argument('--output', default='test-data/layer0_eval_results.json')
    args = parser.parse_args()
    
    # Load scenarios
    with open(args.test_set) as f:
        scenarios = json.load(f)
    print(f"Loaded {len(scenarios)} scenarios")
    
    # Open DB
    conn = sqlite3.connect(args.db)
    
    # Show Layer 0 stats
    cursor = conn.cursor()
    print("\nLayer 0 Statistics:")
    for table in ['gmp_terms', 'gmp_synonyms', 'doc_entities', 'doc_relations', 'doc_taxonomy', 'doc_relevance']:
        cursor.execute(f'SELECT COUNT(*) FROM {table}')
        print(f"  {table}: {cursor.fetchone()[0]:,} rows")
    
    # Run evaluation
    print("\nRunning evaluation...")
    start = time.time()
    results = evaluate_scenarios(conn, scenarios)
    elapsed = time.time() - start
    
    # Print report
    print(f"\n========== EVALUATION REPORT (Python FTS+Layer0) ==========")
    print(f"Total: {results['total']} | Passed: {results['passed']} | Failed: {results['total'] - results['passed']}")
    print(f"Pass Rate: {results['pass_rate']:.1f}%")
    print(f"Avg Recall@K: {results['avg_recall']:.3f}")
    print(f"Avg MRR: {results['avg_mrr']:.3f}")
    print(f"Evaluation time: {elapsed:.1f}s")
    print()
    
    print("Per-Category Results:")
    cats = sorted(results['category_stats'].items())
    for cat, stats in cats:
        rate = (stats['passed'] / stats['total']) * 100 if stats['total'] > 0 else 0
        print(f"  {cat:20s}: {stats['passed']}/{stats['total']} ({rate:.0f}%)")
    
    print("=" * 56)
    
    # Save results
    with open(args.output, 'w') as f:
        json.dump(results, f, ensure_ascii=False, indent=2)
    print(f"\nResults saved to {args.output}")
    
    conn.close()


if __name__ == '__main__':
    main()
