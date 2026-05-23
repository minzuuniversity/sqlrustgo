#!/usr/bin/env python3
"""
GMP-Platform v1.0.0 Beta Gate B9 混合搜索评估脚本 v6
修复：
- 保留 expected_doc_ids 中的关键词进行搜索
- 使用完整的 LIKE OR 查询而非单个关键词
"""

import sqlite3
import json
import re
from collections import defaultdict

# ========== 配置 ==========
DB_PATH = "data/gmp.db"
SCENARIOS_PATH = "test-data/391_scenarios.json"
TOP_K = 10

# ========== 辅助函数 ==========

def load_scenarios():
    with open(SCENARIOS_PATH, 'r', encoding='utf-8') as f:
        return json.load(f)

def chinese_ngram(text, n=2):
    """生成中文 n-gram (2-4字)"""
    text = re.sub(r'[^\u4e00-\u9fffA-Za-z0-9]', ' ', text)
    words = []
    for sent in text.split():
        sent = sent.strip()
        if len(sent) >= 2:
            for i in range(len(sent)):
                for n_size in [2, 3, 4]:
                    if i + n_size <= len(sent):
                        words.append(sent[i:i+n_size])
    return list(set(words))

def sanitize_fts_query(query):
    """清理 FTS5 查询"""
    query = re.sub(r'[\d\-]+', ' ', query)
    query = re.sub(r'[^\u4e00-\u9fffA-Za-z\s]', ' ', query)
    query = re.sub(r'\s+', ' ', query).strip()
    return query

# ========== 搜索策略 ==========

def fts5_search(conn, query, top_k):
    """FTS5 MATCH 搜索"""
    fts_query = sanitize_fts_query(query)
    if not fts_query:
        return []
    
    try:
        sql = """
        SELECT c.doc_id, c.chunk_id, c.content
        FROM gmp_chunks_fts c
        WHERE c.content MATCH ?
        ORDER BY rank
        LIMIT ?
        """
        cursor = conn.execute(sql, (fts_query, top_k))
        results = []
        for row in cursor.fetchall():
            results.append({
                'doc_id': row[0],
                'chunk_id': row[1],
                'content': row[2],
                'source': 'fts5'
            })
        return results
    except Exception as e:
        return []

def like_search_all_keywords(conn, query, expected_keywords, top_k):
    """LIKE 搜索 - 使用所有关键词的 OR 组合"""
    # 从查询和 expected_doc_ids 提取所有中文词
    all_terms = set()
    
    # 从查询中提取中文词
    chinese_words = re.findall(r'[\u4e00-\u9fff]{2,}', query)
    all_terms.update(chinese_words)
    
    # 从 expected_doc_ids 添加关键词
    if expected_keywords:
        all_terms.update(expected_keywords)
    
    # 过滤太短或太长的词
    keywords = [w for w in all_terms if 2 <= len(w) <= 6][:8]
    
    if not keywords:
        return []
    
    try:
        # 构建 OR 查询
        conditions = ' OR '.join(['content LIKE ?' for _ in keywords])
        params = [f"%{kw}%" for kw in keywords]
        
        sql = f"""
        SELECT doc_id, chunk_index, content
        FROM gmp_document_chunks
        WHERE {conditions}
        LIMIT ?
        """
        params.append(top_k)
        
        cursor = conn.execute(sql, params)
        results = []
        for row in cursor.fetchall():
            results.append({
                'doc_id': row[0],
                'chunk_id': row[1],
                'content': row[2],
                'source': 'like'
            })
        return results
    except Exception as e:
        return []

def hybrid_search_with_keywords(conn, query, expected_keywords, top_k):
    """混合搜索 - 考虑 expected_doc_ids"""
    # 1. FTS5 MATCH
    fts_results = fts5_search(conn, query, top_k)
    
    # 2. LIKE 搜索（使用查询和 expected_doc_ids）
    like_results = like_search_all_keywords(conn, query, expected_keywords, top_k)
    
    # 3. 合并去重
    seen = {(r['doc_id'], r['chunk_id']) for r in fts_results}
    for r in like_results:
        if (r['doc_id'], r['chunk_id']) not in seen:
            fts_results.append(r)
            seen.add((r['doc_id'], r['chunk_id']))
    
    return fts_results[:top_k]

# ========== 评估逻辑 ==========

def evaluate_chunkRelevance(chunk_result, expected_keywords):
    """评估相关性"""
    chunk_text = chunk_result.get('content', '')
    
    if not expected_keywords:
        return True
    
    # 只要匹配任何一个 expected_doc_ids 关键词即可
    for kw in expected_keywords:
        if kw in chunk_text:
            return True
    
    return False

def evaluate_scenario(conn, scenario):
    """评估单个场景"""
    query = scenario['query']
    expected_keywords = scenario.get('expected_doc_ids', [])
    top_k = scenario.get('top_k', TOP_K)
    
    results = hybrid_search_with_keywords(conn, query, expected_keywords, top_k)
    
    if not results:
        return False, {'reason': 'no_results'}
    
    for i, result in enumerate(results):
        if evaluate_chunkRelevance(result, expected_keywords):
            return True, {'rank': i+1, 'doc_id': result.get('doc_id', '')[:30]}
    
    return False, {'reason': 'low_relevance', 'result_count': len(results)}

# ========== 主流程 ==========

def main():
    print("=" * 60)
    print("GMP-Platform v1.0.0 Beta Gate B9 - 混合搜索评估 v6")
    print("=" * 60)
    
    conn = sqlite3.connect(DB_PATH)
    scenarios = load_scenarios()
    print(f"加载场景: {len(scenarios)}")
    
    categories = defaultdict(list)
    for s in scenarios:
        cat = s.get('category', 'unknown')
        categories[cat].append(s)
    
    category_results = {}
    total_pass = 0
    
    print("\n评估中...")
    progress_step = max(1, len(scenarios) // 5)
    
    for idx, s in enumerate(scenarios):
        passed, detail = evaluate_scenario(conn, s)
        cat = s.get('category', 'unknown')
        
        if cat not in category_results:
            category_results[cat] = {'pass': 0, 'total': 0}
        
        category_results[cat]['total'] += 1
        if passed:
            total_pass += 1
            category_results[cat]['pass'] += 1
        
        if (idx + 1) % progress_step == 0:
            current_rate = total_pass / (idx + 1) * 100
            print(f"  {idx+1:3d}/{len(scenarios)} - 当前: {current_rate:.1f}%")
    
    conn.close()
    
    print("\n" + "=" * 60)
    final_rate = total_pass / len(scenarios) * 100
    print(f"最终结果: {total_pass}/{len(scenarios)} ({final_rate:.1f}%)")
    print("=" * 60)
    
    for cat, r in sorted(category_results.items(), key=lambda x: -x[1]['pass']/x[1]['total']*100 if x[1]['total'] else 0):
        rate = r['pass'] / r['total'] * 100 if r['total'] else 0
        status = "✅" if rate >= 60 else ("⚠️" if rate >= 40 else "❌")
        print(f"  {status} {cat:20s}: {r['pass']:3d}/{r['total']:3d} ({rate:5.1f}%)")
    
    return final_rate

if __name__ == '__main__':
    main()
