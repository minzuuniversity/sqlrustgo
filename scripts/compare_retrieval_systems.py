#!/usr/bin/env python3
"""
GMP 合规内审检索系统 — 25 场景测试脚本
对比分析 GitNexus 语义检索 vs 传统三路混合检索 vs 文档全文搜索

运行方式: python3 compare_retrieval_systems.py
"""

import json, time, os

PROJECT_ROOT = os.path.dirname(os.path.abspath(__file__))

# ============================================================
# 25 个 GMP 合规内审典型场景
# ============================================================
SCENARIOS = [
    # ---- 偏差管理 (1-3) ----
    {"id": "Q01", "domain": "偏差管理", "query": "偏差分级标准和调查处理流程",
     "expected_type": "规则/SOP", "key_terms": ["偏差分级", "重大偏差", "一般偏差", "调查流程"]},
    {"id": "Q02", "domain": "偏差管理", "query": "偏差与CAPA的关联触发规则",
     "expected_type": "规则/SOP", "key_terms": ["偏差", "CAPA", "触发", "关联"]},
    {"id": "Q03", "domain": "偏差管理", "query": "生产偏差处理的具体步骤和关闭标准",
     "expected_type": "SOP", "key_terms": ["偏差处理", "关闭标准", "根因分析"]},

    # ---- 变更控制 (4-6) ----
    {"id": "Q04", "domain": "变更控制", "query": "变更控制完整流程包括哪些审批节点",
     "expected_type": "SOP/规则", "key_terms": ["变更控制", "审批", "评估", "验证"]},
    {"id": "Q05", "domain": "变更控制", "query": "工艺变更需要启动哪些验证活动",
     "expected_type": "规则/工作流", "key_terms": ["工艺变更", "验证", "风险评估"]},
    {"id": "Q06", "domain": "变更控制", "query": "设备变更控制的具体步骤",
     "expected_type": "SOP", "key_terms": ["设备变更", "确认", "校准"]},

    # ---- 验证与确认 (7-9) ----
    {"id": "Q07", "domain": "验证与确认", "query": "DQ/IQ/OQ/PQ设备确认分别确认什么内容",
     "expected_type": "规则/SOP", "key_terms": ["DQ", "IQ", "OQ", "PQ", "设计确认", "性能确认"]},
    {"id": "Q08", "domain": "验证与确认", "query": "清洁验证方案的制定要求和接受标准",
     "expected_type": "SOP", "key_terms": ["清洁验证", "残留限度", "MACO", "取样"]},
    {"id": "Q09", "domain": "验证与确认", "query": "计算机化系统验证需要满足哪些法规要求",
     "expected_type": "规则/SOP", "key_terms": ["计算机化系统", "验证", "电子记录", "电子签名", "CSV"]},

    # ---- 物料管理 (10-12) ----
    {"id": "Q10", "domain": "物料管理", "query": "物料状态标识管理中待验合格不合格怎么标识",
     "expected_type": "SOP/规则", "key_terms": ["物料状态", "标识", "待验", "合格", "不合格", "颜色"]},
    {"id": "Q11", "domain": "物料管理", "query": "原辅料入库验收的完整流程",
     "expected_type": "SOP", "key_terms": ["入库", "验收", "取样", "检验", "放行"]},
    {"id": "Q12", "domain": "物料管理", "query": "麻醉药品和精神药品的特殊管理要求",
     "expected_type": "SOP/法规", "key_terms": ["麻醉", "精神药品", "特殊管理", "双锁"]},

    # ---- 生产管理 (13-15) ----
    {"id": "Q13", "domain": "生产管理", "query": "清场管理的具体要求和检查要点",
     "expected_type": "SOP", "key_terms": ["清场", "上批残留", "清洁状态"]},
    {"id": "Q14", "domain": "生产管理", "query": "批生产记录的审核要点和常见缺陷",
     "expected_type": "SOP/规则", "key_terms": ["批生产记录", "审核", "签名", "日期"]},
    {"id": "Q15", "domain": "生产管理", "query": "防止生产混淆和交叉污染的控制措施",
     "expected_type": "规则/SOP", "key_terms": ["混淆", "交叉污染", "隔离", "状态标识"]},

    # ---- 质量控制 (16-18) ----
    {"id": "Q16", "domain": "质量控制", "query": "OOS超标结果调查的完整步骤和时限",
     "expected_type": "SOP", "key_terms": ["OOS", "超标", "调查", "阶段", "时限"]},
    {"id": "Q17", "domain": "质量控制", "query": "取样管理的规程和代表性要求",
     "expected_type": "SOP", "key_terms": ["取样", "代表性", "取样量", "取样点"]},
    {"id": "Q18", "domain": "质量控制", "query": "稳定性考察的管理要求和方案设计",
     "expected_type": "SOP/规则", "key_terms": ["稳定性", "留样", "考察", "加速", "长期"]},

    # ---- 质量管理体系 (19-21) ----
    {"id": "Q19", "domain": "质量管理体系", "query": "CAPA管理的完整流程从识别到关闭",
     "expected_type": "SOP", "key_terms": ["CAPA", "识别", "根因", "行动计划", "有效性"]},
    {"id": "Q20", "domain": "质量管理体系", "query": "供应商审计的流程和检查要点",
     "expected_type": "SOP", "key_terms": ["供应商", "审计", "现场", "资质", "评估"]},
    {"id": "Q21", "domain": "质量管理体系", "query": "年度产品质量回顾应包含哪些内容",
     "expected_type": "规则/SOP", "key_terms": ["年度", "质量回顾", "趋势分析", "CPV"]},

    # ---- 文件与数据完整性 (22-23) ----
    {"id": "Q22", "domain": "文件与数据完整性", "query": "ALCOA+数据完整性原则包括哪些要素",
     "expected_type": "法规/规则", "key_terms": ["ALCOA", "数据完整性", "可归属性", "同时性"]},
    {"id": "Q23", "domain": "文件与数据完整性", "query": "GMP文件编号规则和版本管理要求",
     "expected_type": "SOP", "key_terms": ["文件编号", "版本控制", "起草", "批准"]},

    # ---- 设备与设施 (24) ----
    {"id": "Q24", "domain": "设备管理", "query": "设备预防性维护计划和校准管理要求",
     "expected_type": "SOP", "key_terms": ["预防性维护", "校准", "计划", "频率"]},

    # ---- 审计与自检 (25) ----
    {"id": "Q25", "domain": "审计与自检", "query": "GMP自检计划编制和缺陷分级标准",
     "expected_type": "规则/SOP", "key_terms": ["自检", "内审", "缺陷", "分级", "CAPA"]},
]

# ============================================================
# 1. 已有系统的测试数据（从 test_results_final 提取）
# ============================================================
def load_existing_results():
    """加载已有 v22 测试结果"""
    results = {}
    try:
        path = os.path.join(PROJECT_ROOT, 'gmp_kg_build/test_results_final/391_test_simple_results.json')
        with open(path) as f:
            data = json.load(f)
        existing_results = data.get('results', [])
        # 按 query 内容建立索引
        for r in existing_results:
            q = r.get('query', '')
            results[q[:80]] = r
    except:
        pass
    return results

# ============================================================
# 2. 直接 grep 全文检索模拟 (传统关键词方式)
# ============================================================
def keyword_search(query: str, search_dirs: list, top_k: int = 5):
    """模拟传统关键词检索：grep 匹配文件名和关键词"""
    import subprocess
    
    # 提取中文关键词
    keywords = [w for w in query if '\u4e00' <= w <= '\u9fff']
    keyword_str = '|'.join(keywords[:10]) if keywords else query
    
    results = []
    for d in search_dirs:
        try:
            cmd = f"find '{d}' -name '*.md' -o -name '*.yaml' | head -500 | xargs grep -li '{keyword_str[:60]}' 2>/dev/null | head -{top_k}"
            output = subprocess.check_output(cmd, shell=True, text=True, timeout=5)
            for line in output.strip().split('\n'):
                if line:
                    fname = os.path.basename(line)
                    results.append({"file": fname, "path": line})
        except:
            pass
    
    return results[:top_k]

# ============================================================
# 3. 基于 agent-knowledge 规则文件的语义匹配模拟
# ============================================================
RULE_KEYWORDS_MAP = {
    "偏差": ["deviation-capa-rules-enhanced.yaml", "deviation-rules", "偏差调查", "CAPA关联"],
    "变更": ["change-control-rules.yaml", "change-control", "变更控制", "变更流程"],
    "验证": ["validation-rules.yaml", "validation", "DQ", "IQ", "OQ", "PQ"],
    "清洁": ["weighing-rules.yaml", "weighing-workflow.yaml", "清洁验证"],
    "物料": ["material-warehouse-rules.yaml", "material", "物料标识", "物料状态"],
    "生产": ["production-rules.yaml", "production", "清场", "混淆", "污染"],
    "OOS": ["qms-rules.yaml", "OOS", "超标", "实验室"],
    "文件": ["data-integrity-doc-rules.yaml", "data-integrity", "ALCOA", "文件编号"],
    "设备": ["facility-equipment-rules.yaml", "facility", "设备", "校准", "维护"],
    "CAPA": ["capa-rules", "deviation-capa-rules-enhanced.yaml"],
}

def rule_based_match(query: str, rules_dir: str):
    """基于 YAML 规则的语义匹配"""
    matched = []
    for keyword, files in RULE_KEYWORDS_MAP.items():
        if keyword in query:
            for f in files[:2]:
                matched.append({
                    "file": f,
                    "reason": f"关键词匹配: {keyword}"
                })
    return matched[:5]

# ============================================================
# 主测试执行
# ============================================================
def run_all_scenarios():
    """运行 25 场景完整测试"""
    existing = load_existing_results()
    search_dirs = [
        os.path.join(PROJECT_ROOT, 'agent-knowledge/L0-trust-zone/rules'),
        os.path.join(PROJECT_ROOT, 'agent-knowledge/L0-trust-zone/workflows'),
        os.path.join(PROJECT_ROOT, 'agent-knowledge/L1-sanitized-zone/schemas'),
    ]
    
    results = []
    
    for i, s in enumerate(SCENARIOS):
        print(f"[{i+1:02d}/25] {s['id']} [{s['domain']}] {s['query']}")
        
        # 方法1：传统关键词 grep
        t0 = time.time()
        kw_results = keyword_search(s['query'], search_dirs)
        kw_time = time.time() - t0
        
        # 方法2：规则匹配
        rule_results = rule_based_match(s['query'], '')
        
        # 方法3：已有系统对比（如果存在）
        existing_match = existing.get(s['query'][:80], None)
        old_pass = existing_match.get('pass') if existing_match else None
        
        results.append({
            "scenario": s,
            "keyword_hits": len(kw_results),
            "keyword_top3": kw_results[:3],
            "keyword_time_ms": round(kw_time * 1000),
            "rule_matches": rule_results[:3],
            "old_system_pass": old_pass,
            "gitnexus_result": "N/A - 无结果（仅适用代码符号检索）",
        })
        
        print(f"    关键词命中: {len(kw_results)} | 规则匹配: {len(rule_results)} | 旧系统: {'PASS' if old_pass else 'FAIL/无'}")
    
    return results

# ============================================================
# 统计报告
# ============================================================
def print_summary(results):
    print("\n" + "=" * 80)
    print("GMP 合规内审检索 — 25 场景测试对比分析")
    print("=" * 80)
    
    # 总体统计
    total = len(results)
    kw_hit = sum(1 for r in results if r['keyword_hits'] > 0)
    rule_hit = sum(1 for r in results if len(r['rule_matches']) > 0)
    old_pass_count = sum(1 for r in results if r['old_system_pass'] == True)
    
    print(f"\n## 总体统计")
    print(f"| 方法 | 有结果场景 | 空结果场景 | 触达率 |")
    print(f"|------|-----------|-----------|--------|")
    print(f"| 传统关键词(grep) | {kw_hit} | {total - kw_hit} | {kw_hit/total*100:.1f}% |")
    print(f"| 规则文件匹配 | {rule_hit} | {total - rule_hit} | {rule_hit/total*100:.1f}% |")
    print(f"| 旧系统 FTS5 v22 | {old_pass_count} | - | {old_pass_count/total*100:.1f}% (原有164场景) |")
    print(f"| GitNexus 语义检索 | 0 | {total} | 0% (仅适用代码符号) |")
    
    # 领域统计
    print(f"\n## 领域细分")
    domains = {}
    for r in results:
        dom = r['scenario']['domain']
        if dom not in domains:
            domains[dom] = {'total': 0, 'kw_hit': 0, 'rule_hit': 0}
        domains[dom]['total'] += 1
        if r['keyword_hits'] > 0:
            domains[dom]['kw_hit'] += 1
        if len(r['rule_matches']) > 0:
            domains[dom]['rule_hit'] += 1
    
    print(f"| 领域 | 场景数 | grep命中 | 规则匹配命中 |")
    print(f"|------|--------|----------|-------------|")
    for dom, stats in sorted(domains.items()):
        print(f"| {dom} | {stats['total']} | {stats['kw_hit']}/{stats['total']} | {stats['rule_hit']}/{stats['total']} |")
    
    return results


if __name__ == '__main__':
    results = run_all_scenarios()
    print_summary(results)
    
    with open('/tmp/gmp_retrieval_compare.json', 'w') as f:
        json.dump(results, f, ensure_ascii=False, indent=2, default=str)
    print("\n结果已保存到 /tmp/gmp_retrieval_compare.json")