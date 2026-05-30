#!/usr/bin/env python3
"""
SSOT (Single Source of Truth) 重复内容检查脚本
检查多个文档中是否存在重复内容

Truthfulness 原则：
- 禁止在多个位置写相同内容
- 禁止从非 SSOT 来源复制内容
- 文档内容必须与 SSOT 保持一致
"""

import sys
import re
from pathlib import Path
from collections import defaultdict

def extract_meaningful_text(content):
    """提取有意义的文本内容，去除链接、代码块等"""
    content = re.sub(r'\[([^\]]+)\]\([^\)]+\)', r'\1', content)
    content = re.sub(r'!\[([^\]]*)\]\([^\)]+\)', '', content)
    content = re.sub(r'```[\s\S]*?```', '', content)
    content = re.sub(r'`[^`]+`', '', content)
    content = re.sub(r'<[^>]+>', '', content)
    content = re.sub(r'^#+\s*', '', content)
    content = re.sub(r'\|[-:\s]+\|?', '', content)
    content = re.sub(r'\n{3,}', '\n\n', content)
    return content.strip()

def get_documents(directory):
    """获取目录下所有 Markdown 文件"""
    docs = {}
    dir_path = Path(directory)
    for md_file in dir_path.rglob('*.md'):
        try:
            content = md_file.read_text(encoding='utf-8')
            docs[str(md_file)] = content
        except Exception as e:
            print(f"Warning: Cannot read {md_file}: {e}", file=sys.stderr)
    return docs

def compute_similarity(text1, text2):
    """计算两个文本的相似度"""
    words1 = set(re.findall(r'\b\w{4,}\b', text1.lower()))
    words2 = set(re.findall(r'\b\w{4,}\b', text2.lower()))
    if not words1 or not words2:
        return 0.0
    intersection = words1 & words2
    union = words1 | words2
    return len(intersection) / len(union) if union else 0.0

def find_duplicates(docs, threshold=0.7):
    """找出相似度超过阈值的文档对"""
    duplicates = []
    paths = list(docs.keys())
    
    for i in range(len(paths)):
        for j in range(i + 1, len(paths)):
            text1 = extract_meaningful_text(docs[paths[i]])
            text2 = extract_meaningful_text(docs[paths[j]])
            
            similarity = compute_similarity(text1, text2)
            if similarity >= threshold:
                duplicates.append({
                    'file1': paths[i],
                    'file2': paths[j],
                    'similarity': similarity,
                    'preview': text2[:200] + '...' if len(text2) > 200 else text2
                })
    
    return duplicates

def main():
    import argparse
    parser = argparse.ArgumentParser(description='SSOT 重复内容检查')
    parser.add_argument('--dir', '-d', default='docs/releases', help='文档目录')
    parser.add_argument('--threshold', '-t', type=float, default=0.7, help='相似度阈值 (0.0-1.0)')
    parser.add_argument('--verbose', '-v', action='store_true', help='详细输出')
    args = parser.parse_args()
    
    if not Path(args.dir).exists():
        print(f"Error: Directory '{args.dir}' not found", file=sys.stderr)
        sys.exit(1)
    
    print(f"扫描文档目录: {args.dir}")
    print(f"相似度阈值: {args.threshold}")
    print("-" * 60)
    
    docs = get_documents(args.dir)
    print(f"找到 {len(docs)} 个文档")
    
    duplicates = find_duplicates(docs, args.threshold)
    
    if duplicates:
        print(f"\n⚠️  发现 {len(duplicates)} 对重复/高度相似文档:\n")
        for dup in sorted(duplicates, key=lambda x: -x['similarity']):
            print(f"📄 {dup['file1']}")
            print(f"📄 {dup['file2']}")
            print(f"   相似度: {dup['similarity']:.1%}")
            if args.verbose:
                print(f"   内容预览: {dup['preview'][:100]}...")
            print()
        sys.exit(1)
    else:
        print("\n✅ 未发现重复内容 (SSOT 合规)")
        sys.exit(0)

if __name__ == '__main__':
    main()