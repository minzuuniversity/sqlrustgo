#!/usr/bin/env python3
'''SRG-D004-01 验收自动化：RED/GREEN/REGRESSION/MUTATION_RED 四判据确定性执行。

用法（在 srg-test-factory-pilot 根目录）：
    python3 tools/verify_srg_d004_acceptance.py --repo . --phase pre   # 修复前：RED 成立判据
    python3 tools/verify_srg_d004_acceptance.py --repo . --phase post  # 修复后：GREEN+REGRESSION+MUTATION_RED

判据（与 SRG-D004-01.local-text.json 一致）：
  R1 RED 成立: tf_t4_04 FAILED（修复前）/ tf_t4_04 PASS（修复后）
  R2 不扩大修复面: 修复后 tf_t4_01/02/03 仍 FAILED
  R3 回归: storage lib 全量（复跑基线过滤族）无新增 FAIL
  R4 MUTATION_RED: 独立副本恢复 Ok(0) → tf_t4_04 再次 FAILED
退出码：0=判据全部成立；1=任一判据失败（fail-closed，不静默）。
'''
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

PACKAGE = 'sqlrustgo-storage'
TEST = 'tf_t4_stub_silent_success'


def run_cargo_tests(repo: Path, target: str, filters: list[str]) -> tuple[int, list[tuple[str, str]]]:
    cmd = ['cargo', 'test', '-p', PACKAGE, '--test', target, '--'] + filters if target else \
          ['cargo', 'test', '-p', PACKAGE, '--lib', '--'] + filters
    proc = subprocess.run(cmd, cwd=repo, capture_output=True, text=True, timeout=2400)
    raw = proc.stdout + proc.stderr
    tests = re.findall(r'^test\s+([\w:]+)\s+\.\.\.\s+(ok|FAILED|ignored)', raw, re.M)
    return proc.returncode, tests


def status_map(tests: list[tuple[str, str]]) -> dict[str, str]:
    return dict(tests)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument('--repo', default='.')
    ap.add_argument('--phase', choices=['pre', 'post'], required=True)
    args = ap.parse_args()
    repo = Path(args.repo).resolve()
    failures: list[str] = []
    report: dict = {'phase': args.phase, 'base_sha': subprocess.run(
        ['git', '-C', str(repo), 'rev-parse', 'HEAD'], capture_output=True, text=True).stdout.strip()}

    rc, tests = run_cargo_tests(repo, TEST, [])
    sm = status_map(tests)
    t04 = sm.get('tf_t4_04_non_tx_backend_must_reject_or_document')
    report['tf_t4_04'] = t04

    if args.phase == 'pre':
        if rc != 101 or t04 != 'FAILED':
            failures.append(f'R1 RED not established: rc={rc} tf_t4_04={t04}')
    else:
        if rc != 0 or t04 != 'ok':
            failures.append(f'R1 GREEN not met: rc={rc} tf_t4_04={t04}')
        # R2 不扩大修复面
        for name in ('tf_t4_01_scan_after_insert_sees_rows', 'tf_t4_02_delete_reports_real_count',
                     'tf_t4_03_update_reports_real_count'):
            if sm.get(name) == 'ok':
                failures.append(f'R2 scope violation: {name} unexpectedly passed (must stay FAILED)')
        # R3 回归（lib 全量抽查族）
        rc3, _ = run_cargo_tests(repo, None, ['undo', 'rollback', 'savepoint', 'transaction'])
        if rc3 != 0:
            failures.append(f'R3 regression: lib filters rc={rc3}')
        # R4 MUTATION_RED：独立副本恢复 Ok(0)
        with tempfile.TemporaryDirectory() as td:
            mutant = Path(td) / 'mutant.rs'
            src = (repo / 'crates/storage/tests' / (TEST + '.rs')).read_text()
            mutant.write_text(src)
            # 用 git worktree 做真正的独立副本（不污染候选提交）
            wt = Path(td) / 'wt'
            subprocess.run(['git', '-C', str(repo), 'worktree', 'add', str(wt), 'HEAD'],
                           capture_output=True, text=True)
            bf = wt / 'crates/storage/src/binary_storage_v2.rs'
            body = bf.read_text()
            mutated = body.replace(
                'fn begin_transaction(&mut self) -> SqlResult<u64> {\n        Ok(0)\n    }',
                'fn begin_transaction(&mut self) -> SqlResult<u64> {\n        Ok(1)\n    }')
            if mutated == body:
                failures.append('R4 mutation did not apply (pattern mismatch; verify source unchanged)')
            else:
                bf.write_text(mutated)
                rc4, t4 = run_cargo_tests(wt, TEST, ['tf_t4_04'])
                s4 = status_map(t4).get('tf_t4_04_non_tx_backend_must_reject_or_document')
                if not (rc4 == 101 and s4 == 'FAILED'):
                    failures.append(f'R4 MUTATION_RED failed: rc={rc4} tf_t4_04={s4} (test cannot detect revert!)')
                subprocess.run(['git', '-C', str(wt), 'worktree', 'remove', '--force', str(wt)],
                               capture_output=True, text=True)

    report['failures'] = failures
    print(json.dumps(report, ensure_ascii=False, indent=1))
    return 1 if failures else 0


if __name__ == '__main__':
    sys.exit(main())
