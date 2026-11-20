#!/usr/bin/env python3
"""Mechanical checks of HARA deliverables and captured observations, not a DB gate."""
import collections
import datetime
import hashlib
import json
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
AUDIT = HERE.parent
ROOT = HERE.parents[3]

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    reports = sorted(AUDIT.glob('0[1-9]_*.md'))
    assert len(reports) == 9, len(reports)
    checks = {'reports': len(reports), 'priority_sets': {}, 'local_links': 0}
    for path in reports:
        body = path.read_text()
        for field in ['source_agent', 'source_run', 'timestamp', 'evidence_hash', 'conflict_resolution']:
            assert field in body, (path, field)
        for link in re.findall(r'\]\(([^)]+)\)', body):
            if not re.match(r'\w+://', link):
                assert (path.parent / link.split('#')[0]).exists(), (path, link)
                checks['local_links'] += 1
        for digest in re.findall(r'sha256:([0-9a-f]{64})', body):
            assert digest in {sha(p) for p in HERE.glob('*.json')}, (path, digest)
    data = json.loads((HERE/'gitea-issues.json').read_text())['data']
    assert len({x['number'] for x in data}) == len(data) == 104
    assert all(x.get('pull_request') is None for x in data)
    groups = json.loads((HERE/'issue_groups.json').read_text())
    for priority, mapping in groups.items():
        actual = {x['number'] for x in data if any(l['name'] == priority for l in x['labels'])}
        flat = [n for ids in mapping.values() for n in ids]
        assert len(flat) == len(set(flat)), collections.Counter(flat)
        assert actual == set(flat), {'missing': sorted(actual-set(flat)), 'extra': sorted(set(flat)-actual)}
        checks['priority_sets'][priority] = {'count': len(actual), 'missing': [], 'extra': [], 'duplicates': []}
    run = json.loads((HERE/'probe-run6.json').read_text())
    assert run['exit'] == 0
    for observation in [
        'HARA bufferpool_rollback expected=0 actual=1',
        'HARA commit_sync lockfree=false expected_at_least=1 actual=1',
        'HARA commit_sync lockfree=true expected_at_least=1 actual=0',
        'HARA recovery_failure_first expected=Err actual=Err',
        'HARA recovery_failure_second expected=Err actual=Ok',
        'HARA flush_insert_rollback flush=false expected=0 actual=0',
        'HARA flush_insert_rollback flush=true expected=0 actual=1',
        'HARA undo_failure expected=Err actual=Ok(())',
        'HARA sql_replace dml=Ok(()) rollback=Ok(()) expected=[[1,10]] actual=Ok([])',
        'HARA sql_odku dml=Ok(()) rollback=Ok(()) expected=[[1,10]] actual=Ok([])',
    ]:
        assert observation in run['output'], observation
    wire = json.loads((HERE/'wire-rollback-full.json').read_text())
    assert wire['exit'] == 0 and '6 passed; 0 failed; 0 ignored' in wire['output'], wire['output']
    assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip() == 'e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0'
    changed = subprocess.check_output(['git','diff','HEAD','--name-only'],cwd=ROOT,text=True).splitlines()
    assert all(x.startswith('docs/audits/HARA-410/') for x in changed), changed
    checks['tracked_changes_outside_audit'] = []
    checks['scope'] = 'Delivery consistency and observed-output assertions only; not database correctness certification'
    print(json.dumps(checks,ensure_ascii=False,indent=2))
    if '--manifest' in sys.argv:
        sources = [
            'src/execution_engine_methods.rs','src/engine_dml.rs','src/engine_helpers.rs',
            'src/savepoint_wiring.rs','src/engine_builder.rs',
            'crates/storage/src/engine.rs','crates/storage/src/file_storage.rs',
            'crates/storage/src/mvcc_storage.rs','crates/storage/src/mvcc.rs',
            'crates/storage/src/wal_storage.rs','crates/storage/src/parallel_wal_storage.rs',
            'crates/storage/src/wal/file_backed_wal_manager.rs','crates/storage/src/wal_legacy.rs',
            'crates/storage/src/recovery_engine.rs','crates/storage/src/checkpoint.rs',
            'crates/storage/src/binary_storage.rs','crates/storage/src/binary_storage_v2.rs',
            'crates/storage/src/append_only_storage.rs','crates/storage/src/table_level_storage.rs',
            'crates/storage/src/vtu_guard.rs','crates/storage/src/buffer_pool.rs',
            'crates/storage/src/clustered_table.rs','crates/storage/src/replication.rs',
            'crates/storage/src/binlog_server.rs','crates/storage/src/pitr.rs',
            'crates/transaction/src/transaction_manager.rs','crates/transaction/src/savepoint.rs',
            'crates/executor/src/trigger.rs','crates/mysql-server/src/lib.rs',
            'Cargo.toml','Cargo.lock','AGENTS.md','docs/releases/v4.1.0/STAGE.yaml'
        ]
        manifest = {
            'source_agent':'Codex GPT-6.1 Sol','source_run':'HARA-410-20261120',
            'timestamp':datetime.datetime.now(datetime.timezone.utc).isoformat(),
            'commit':'e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0',
            'tree':'4194fc975ebaf749de477f42ca5a300facef1278',
            'conflict_resolution':'source-bound advice; owner decides architecture and acceptance',
            'source_sha256':{p:sha(ROOT/p) for p in sources},
            'files':{str(p.relative_to(AUDIT)):sha(p) for p in sorted(AUDIT.rglob('*'))
                     if p.is_file() and p.name != 'MANIFEST.json' and '__pycache__' not in p.parts}
        }
        (HERE/'MANIFEST.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n')
        print(json.dumps({'manifest_sha256':sha(HERE/'MANIFEST.json'),
                          'hashed_delivery_files':len(manifest['files'])},ensure_ascii=False))

if __name__ == '__main__':
    main()
