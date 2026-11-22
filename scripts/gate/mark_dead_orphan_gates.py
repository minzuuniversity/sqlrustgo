# -*- coding: utf-8 -*-
"""给 dead / orphan 两类门禁脚本加**文件头自标**。

为什么
----
`docs/releases/v4.1.0/GATE_CATALOG.jsonl` 已经把每一道 dead / orphan 门禁的
`disposition` / `note` / `evidence` / `effect` 登记齐了（`gate_registry.py check`
⇒ fails: 0）。但**登记表是另一份文件** —— 单独打开某个门禁脚本时，读者看不到
「它其实不会被执行」这件事 ⇒ 容易误以为它在生效。

本脚本把登记结论**就地写进文件头**，让每个脚本自解释：
分类（dead / orphan）、处置结论、owner task、以及「为什么保留文件而不是删掉」。

边界（刻意收紧）
----------------
1. **只加注释，不改任何可执行逻辑** —— 在 shebang 之后插入注释块，正文一字不动。
2. **注释里不写任何其它门禁脚本的文件名** —— `gate_registry.py` 的
   `orphan_referenced` 分类虽然按「去注释后的正文引用」算，但别的检查
   （如按 grep 扫引用的门禁）未必如此 ⇒ 保守起见不引入任何新的脚本名。
3. **幂等** —— 用标记串 `GATE-DISPOSITION-SELF-MARK` 检测，重复运行不重复插入。
4. **不改门禁判据、阈值、分母** —— 门禁脚本总数 168 不变。

用法
----
    python3 scripts/gate/mark_dead_orphan_gates.py --apply     # 真正写入
    python3 scripts/gate/mark_dead_orphan_gates.py --dry-run   # 只报告
"""
from __future__ import annotations

import argparse
import io
import json
import os
import sys

REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
CATALOG = os.path.join(REPO_ROOT, 'docs/releases/v4.1.0/GATE_CATALOG.jsonl')
MARK = 'GATE-DISPOSITION-SELF-MARK'

DISPOSITION_TEXT = {
    'archive-candidate': '归档候选（archive-candidate）',
    'keep-rewire': '保留并改接（keep-rewire）',
    'needs-execution-point': '待接入执行点（needs-execution-point）',
    'needs-external-host': '需外部主机（needs-external-host）',
    'superseded': '已被取代（superseded）',
}

CLASS_TEXT = {
    'dead': '死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行',
    'orphan_referenced': '孤儿门禁（orphan_referenced）—— 只在其它脚本正文里被引用，没有任何执行点',
}


def load_catalog():
    rows = []
    with io.open(CATALOG, encoding='utf-8') as f:
        for line in f:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def self_mark_block(rec):
    cls = rec.get('class', '')
    dsp = rec.get('disposition', '')
    note = (rec.get('note') or '').replace('\n', ' ').strip()
    effect = (rec.get('effect') or {})
    if isinstance(effect, dict):
        reason = (effect.get('reason') or '').replace('\n', ' ').strip()
    else:
        reason = str(effect).replace('\n', ' ').strip()
    owner = rec.get('owner_task', '')

    lines = [
        '# ' + '=' * 74,
        '# ' + MARK + ' —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里',
        '#',
        '# 分类：%s' % CLASS_TEXT.get(cls, cls),
        '# 处置：%s' % DISPOSITION_TEXT.get(dsp, dsp),
        '# 责任任务：%s' % owner,
    ]
    if note:
        lines.append('# 登记说明：%s' % note)
    if reason:
        lines.append('# 效果声明：%s' % reason)
    lines += [
        '#',
        '# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），',
        '# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么',
        '# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。',
        '#',
        '# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）',
        '# ' + '=' * 74,
    ]
    return lines


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--apply', action='store_true')
    ap.add_argument('--dry-run', action='store_true')
    args = ap.parse_args()

    rows = load_catalog()
    targets = [r for r in rows if r.get('class') in ('dead', 'orphan_referenced')]
    changed = skipped = missing = 0
    report = []

    for rec in targets:
        path = os.path.join(REPO_ROOT, rec['gate'])
        if not os.path.isfile(path):
            missing += 1
            report.append('MISSING  %s' % rec['gate'])
            continue
        with io.open(path, encoding='utf-8') as f:
            src = f.read()
        if MARK in src:
            skipped += 1
            continue
        lines = src.split('\n')
        # 插在 shebang 之后（第一行不是 shebang 时插在最前）
        ins = 0
        if lines and lines[0].startswith('#!'):
            ins = 1
        block = self_mark_block(rec)
        new = lines[:ins] + block + lines[ins:]
        out = '\n'.join(new)
        if args.apply:
            with io.open(path, 'w', encoding='utf-8') as f:
                f.write(out)
        changed += 1
        report.append('MARKED   %s (%s / %s)' % (rec['gate'], rec.get('class'), rec.get('disposition')))

    print('dead+orphan 目标=%d  已改=%d  已标记跳过=%d  文件缺失=%d'
          % (len(targets), changed, skipped, missing))
    if not args.apply:
        print('--- dry-run（未写入）前 10 条 ---')
        for line in report[:10]:
            print(' ', line)
    else:
        print('--- 已写入前 5 条 ---')
        for line in report[:5]:
            print(' ', line)
    return 0


if __name__ == '__main__':
    sys.exit(main())
