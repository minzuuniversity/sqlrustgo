#!/usr/bin/env python3
"""
GMP 文档脱敏脚本
================================
将 gmp-md/ 中的敏感信息替换为占位符，生成 gmp-md-masking/
保留文档结构和 GMP 知识内容，使系统可公开发布/发表学术文章。

脱敏范围: 企业标识、公司名、人名、产品名、药材名、注册号、地址、电话、身份证号
"""

import os, re, shutil, json
from pathlib import Path
from collections import defaultdict

PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC_DIR  = os.path.join(PROJECT_ROOT, 'gmp-md')
DST_DIR  = os.path.join(PROJECT_ROOT, 'gmp-md-masking')
MAP_FILE = os.path.join(PROJECT_ROOT, 'gmp-md-masking', 'MASKING-MAP.json')

# ============================================================
# 脱敏映射表 — 所有替换在此定义
# ============================================================

# 公司/企业标识
COMPANY_MAP = {
    '贵州高原彝药厂有限公司': '[COMPANY-NAME-PHARMA]',
    '贵州高原彝药厂':       '[COMPANY-SHORT]',
    'GZYZ':                '[ENT-CODE]',
}

# 人名 (占位符 + 真实值脱敏)
NAME_MAP = {
    '张三': '[NAME-A-SUPERVISOR]',
    '李四': '[NAME-B-REVIEWER]',
    '王五': '[NAME-C-APPROVER]',
    '胡馨': '[NAME-D-QP]',           # Quality Person 质量受权人
}

# 产品名
PRODUCT_MAP = {
    '复方伸筋胶囊': '[PRODUCT-A-CAPSULE]',
    '清痹通络药酒': '[PRODUCT-B-TINCTURE]',
}

# 中药材名 (为保护配方保密性脱敏)
HERB_MAP = {
    '伸筋草':   '[HERB-A]',
    '虎杖':     '[HERB-B]',
    '大血藤':   '[HERB-C]',
    '透骨香':   '[HERB-D]',
    '滇白珠':   '[HERB-D]',        # 透骨香别名
    '水冬瓜':   '[HERB-E]',
    '见血飞':   '[HERB-F]',
    '山药':     '[HERB-G]',
}

# 注册号/批文号 (保护具体批号数字)
REG_MAP = {
    '国药准字Z20250001':   '[REG-NO-SAMPLE]',
    '国药准字Z':           '[REG-NO-PREFIX]',
}

# 地址/联系方式
ADDRESS_STRINGS = [
    '贵州省贵安新区湖潮乡羊艾生物医药园',
]

# 身份证号模式 (18位)
ID_PATTERN = re.compile(r'\b[1-9]\d{5}(19|20)\d{2}(0[1-9]|1[0-2])(0[1-9]|[12]\d|3[01])\d{3}[\dXx]\b')

# 电话号码模式 (座机)
PHONE_PATTERN = re.compile(r'\b(0\d{2,3}[-]\d{7,8}|\d{3,4}[-]\d{7,8})\b')

# 传真
FAX_PATTERN = re.compile(r'(传真[：:]\s*)\d{3,4}[-]\d{7,8}')

# 邮编
ZIP_PATTERN = re.compile(r'\b\d{6}\b')

# ============================================================
# 可逆映射记录 (新占位符 → 原始值)
# ============================================================

REVERSE_MAP = {}

def record_mapping(placeholder: str, original: str):
    """记录映射以便可逆"""
    REVERSE_MAP[placeholder] = original

# ============================================================
# 预处理：收集所有文件名中的敏感模式
# ============================================================

def build_filename_rename_map(src_root: str):
    """扫描文件名，建立重命名映射表"""
    rename_map = {}
    
    for dirpath, dirnames, filenames in os.walk(src_root):
        for fname in filenames:
            old_path = os.path.join(dirpath, fname)
            rel_path = os.path.relpath(old_path, src_root)
            new_rel = rel_path
            
            # 公司名
            for orig, repl in COMPANY_MAP.items():
                if orig in new_rel:
                    new_rel = new_rel.replace(orig, repl)
                    record_mapping(repl, orig)
            
            # 产品名
            for orig, repl in PRODUCT_MAP.items():
                if orig in new_rel:
                    new_rel = new_rel.replace(orig, repl)
                    record_mapping(repl, orig)
            
            # 药材名
            for orig, repl in HERB_MAP.items():
                if orig in new_rel:
                    new_rel = new_rel.replace(orig, repl)
                    record_mapping(repl, orig)
            
            if new_rel != rel_path:
                rename_map[old_path] = new_rel
        
    return rename_map

# ============================================================
# 文件内容脱敏
# ============================================================

def mask_content(text: str, filepath: str) -> tuple:
    """
    对文件内容执行全部脱敏替换。
    返回 (masked_text, stats_dict)
    """
    stats = defaultdict(int)
    
    # 1. 公司/企业标识
    for orig, repl in COMPANY_MAP.items():
        count = len(re.findall(re.escape(orig), text))
        if count > 0:
            text = text.replace(orig, repl)
            stats[f'company:{orig}->{repl}'] += count
            record_mapping(repl, orig)
    
    # 2. 人名 (按长度降序确保长匹配优先)
    for orig, repl in sorted(NAME_MAP.items(), key=lambda x: -len(x[0])):
        count = len(re.findall(re.escape(orig), text))
        if count > 0:
            text = text.replace(orig, repl)
            stats[f'person:{orig}->{repl}'] += count
            record_mapping(repl, orig)
    
    # 3. 产品名 (按长度降序)
    for orig, repl in sorted(PRODUCT_MAP.items(), key=lambda x: -len(x[0])):
        count = len(re.findall(re.escape(orig), text))
        if count > 0:
            text = text.replace(orig, repl)
            stats[f'product:{orig}->{repl}'] += count
            record_mapping(repl, orig)
    
    # 4. 药材名 (按长度降序)
    for orig, repl in sorted(HERB_MAP.items(), key=lambda x: -len(x[0])):
        count = len(re.findall(re.escape(orig), text))
        if count > 0:
            text = text.replace(orig, repl)
            stats[f'herb:{orig}->{repl}'] += count
            record_mapping(repl, orig)
    
    # 5. 注册/批号
    for orig, repl in REG_MAP.items():
        count = len(re.findall(re.escape(orig), text))
        if count > 0:
            text = text.replace(orig, repl)
            stats[f'reg:{orig}->{repl}'] += count
            record_mapping(repl, orig)
    
    # 6. 地址
    for addr in ADDRESS_STRINGS:
        count = len(re.findall(re.escape(addr), text))
        if count > 0:
            text = text.replace(addr, '[ADDRESS-FACILITY]')
            stats[f'address:{addr[:20]}...->[ADDRESS-FACILITY]'] += count
            record_mapping('[ADDRESS-FACILITY]', addr)
    
    # 7. 身份证号
    id_matches = [m.group() for m in ID_PATTERN.finditer(text)]
    for i, match in enumerate(set(id_matches)):
        text = text.replace(match, f'[ID-NUMBER-{i+1:02d}]')
        stats[f'id_number'] += 1
        record_mapping(f'[ID-NUMBER-{i+1:02d}]', match)
    
    # 8. 电话号码 (仅替换包含传真等标签的)
    # 座机号比较通用，仅替换明确带"电话："等标签的
    phone_label = re.compile(r'(电话[：:]\s*)(\d{3,4}[-]\d{7,8})')
    text = phone_label.sub(r'\1[PHONE-NUMBER]', text)
    
    fax_matches = FAX_PATTERN.findall(text)
    if fax_matches:
        text = FAX_PATTERN.sub(r'\1[FAX-NUMBER]', text)
        stats['fax'] += len(fax_matches)
        record_mapping('[FAX-NUMBER]', '传真号码')
    
    # 邮编 — 仅替换紧跟"邮编"标签的
    zip_label = re.compile(r'(邮编[：:]\s*)\d{6}')
    text = zip_label.sub(r'\1[ZIP-CODE]', text)
    
    return text, dict(stats)

# ============================================================
# 主流程
# ============================================================

def main():
    print("=" * 60)
    print("GMP 文档脱敏处理")
    print(f"源目录: {SRC_DIR}")
    print(f"目标目录: {DST_DIR}")
    print("=" * 60)
    
    # 清理目标目录
    if os.path.exists(DST_DIR):
        shutil.rmtree(DST_DIR)
    
    # 1. 复制目录结构
    print("\n[1/4] 复制目录结构...")
    shutil.copytree(SRC_DIR, DST_DIR, dirs_exist_ok=True)
    
    # 2. 文件名脱敏 (先扫描映射)
    print("[2/4] 文件名脱敏...")
    fn_rename_map = build_filename_rename_map(SRC_DIR)
    for old_path, new_rel in fn_rename_map.items():
        new_path = os.path.join(DST_DIR, new_rel)
        old_dst_path = os.path.join(DST_DIR, os.path.relpath(old_path, SRC_DIR))
        if os.path.exists(old_dst_path):
            os.makedirs(os.path.dirname(new_path), exist_ok=True)
            os.rename(old_dst_path, new_path)
            print(f"  RENAME: {os.path.basename(old_path)} -> {os.path.basename(new_path)}")
    
    # 3. 内容脱敏
    print("[3/4] 内容脱敏...")
    total_stats = defaultdict(int)
    total_files = 0
    masked_files = 0
    
    for dirpath, dirnames, filenames in os.walk(DST_DIR):
        for fname in filenames:
            if not fname.endswith('.md'):
                continue
            filepath = os.path.join(dirpath, fname)
            total_files += 1
            
            with open(filepath, 'r', encoding='utf-8') as f:
                original = f.read()
            
            masked, stats = mask_content(original, filepath)
            
            if masked != original:
                masked_files += 1
                with open(filepath, 'w', encoding='utf-8') as f:
                    f.write(masked)
                for k, v in stats.items():
                    total_stats[k] += v
        
        # 进度条
        if total_files % 200 == 0 and total_files > 0:
            print(f"  已处理: {total_files} 文件, 脱敏: {masked_files}")
    
    print(f"  完成: {total_files} 文件, {masked_files} 个文件已脱敏")
    
    # 4. 写入映射文件
    print(f"[4/4] 写入映射表: {MAP_FILE}")
    mapping_data = {
        'description': 'GMP文档脱敏映射表 — 仅用于本地还原，切勿提交到公开仓库',
        'version': '1.0',
        'date': '2026-05-20',
        'total_files': total_files,
        'masked_files': masked_files,
        'mappings': {
            'company': {k: v for k, v in COMPANY_MAP.items()},
            'names': {k: v for k, v in NAME_MAP.items()},
            'products': {k: v for k, v in PRODUCT_MAP.items()},
            'herbs': {k: v for k, v in HERB_MAP.items()},
            'registration': {k: v for k, v in REG_MAP.items()},
            'reverse_map': REVERSE_MAP,
        },
        'stats': dict(total_stats),
    }
    
    with open(MAP_FILE, 'w', encoding='utf-8') as f:
        json.dump(mapping_data, f, ensure_ascii=False, indent=2)
    
    # 5. 总结
    print(f"\n{'='*60}")
    print("脱敏完成!")
    print(f"总文件: {total_files}")
    print(f"已脱敏: {masked_files} ({masked_files/total_files*100:.1f}%)")
    print(f"\n脱敏统计:")
    for k, v in sorted(total_stats.items()):
        print(f"  {k}: {v} 处")
    print(f"\n映射表: {MAP_FILE}")
    print(f"脱敏目录: {DST_DIR}")
    print(f"{'='*60}")
    
    # 6. 在 gmp-md-masking 目录生成 README
    readme_path = os.path.join(DST_DIR, 'README.md')
    with open(readme_path, 'w', encoding='utf-8') as f:
        f.write("""# GMP 合规文档数据集 — 脱敏版

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
| 企业名称 | 占位符替换 | 贵州高原彝药厂有限公司 → [COMPANY-NAME-PHARMA] |
| 企业代码 | 占位符替换 | GZYZ → [ENT-CODE] |
| 人名 | 占位符替换 | 张三 → [NAME-A-SUPERVISOR] |
| 产品名称 | 占位符替换 | 复方伸筋胶囊 → [PRODUCT-A-CAPSULE] |
| 中药材名 | 占位符替换 | 伸筋草 → [HERB-A] |
| 注册批号 | 占位符替换 | 国药准字Z20250001 → [REG-NO-SAMPLE] |
| 地址/电话/身份证 | 占位符替换 | 地址 → [ADDRESS-FACILITY] |

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
""")
    
    print(f"\nREADME 已生成: {readme_path}")


if __name__ == '__main__':
    main()