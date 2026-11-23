#!/usr/bin/env python3
"""Offline architecture packet checks; no dispatch, model or service mutation."""
import argparse
import base64
import copy
import datetime
import hashlib
import importlib.util
import json
import pathlib
import re
import subprocess
import sys

sys.dont_write_bytecode=True

PACKET=pathlib.Path(__file__).resolve().parents[1]
ROOT=pathlib.Path(subprocess.check_output(['git','rev-parse','--show-toplevel'],cwd=PACKET,text=True).strip())
BASE='e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0'
TEST='56cfa222f1450975d3b3adc6ab9c36a4d258a5f6'
PARENT='45da6968ae2baaade6a31206eeee5854a8514ee2'
SKIP={'handoff/MANIFEST.json','handoff/SHA256SUMS.txt','evidence/validation.json','evidence/PUBLICATION_RECEIPT.json','evidence/DELIVERY_RECEIPT.json','evidence/independent-pr5915.log'}

def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def digest(s):return hashlib.sha256(s.encode()).hexdigest()
def read(p):return json.loads(p.read_text())
def dump(p,d):
    p.parent.mkdir(parents=True,exist_ok=True)
    p.write_text(json.dumps(d,ensure_ascii=False,indent=2)+'\n')
def content_files():
    return sorted(p for p in PACKET.rglob('*') if p.is_file() and str(p.relative_to(PACKET)) not in SKIP and '__pycache__' not in p.parts)
def reviewed_handoff():
    p=PACKET/'evidence/inputs/dsh_handoff.py'
    s=importlib.util.spec_from_file_location('reviewed_dsh_handoff',p)
    m=importlib.util.module_from_spec(s);sys.modules[s.name]=m;s.loader.exec_module(m)
    return m

def build():
    now=datetime.datetime.now(datetime.timezone.utc).isoformat()
    sources={}
    production=['src/execution_engine_methods.rs','src/engine_builder.rs','crates/storage/src/file_storage.rs','crates/storage/src/wal_storage.rs','crates/storage/src/recovery_engine.rs','crates/storage/src/mvcc_storage.rs','crates/transaction/src/transaction_manager.rs']
    for p in production:
        b=subprocess.check_output(['git','show',BASE+':'+p],cwd=ROOT)
        sources[BASE+':'+p]=hashlib.sha256(b).hexdigest()
    for name in ['ARR_410_ARCHITECTURE_REVIEW.md','ARR_410_REVIEW_RESPONSE.md','ARR_410_B_EXTENDED_ARCHITECTURE_REVIEW.md','ARR_B1_TRANSACTION_RECOVERY.md','ARR_B2_STORAGE_IDENTITY.md','ARR_B3_DML_COVERAGE.md','M_PRE2_EVIDENCE.md','RSC_CRASH_CONTRACT.md','SLOT_LIFECYCLE_ASSESSMENT.md']:
        p=pathlib.Path('/Volumes/workspace/dev/srg-handover-p2a')/name
        sources[str(p)]=sha(p)
    for p in (PACKET/'evidence/inputs').iterdir():
        if p.is_file():sources[str(p.relative_to(PACKET))]=sha(p)
    source={'source_agent':'Codex GPT-6.1 Sol','source_run':'RUNTIME-5915-20261120','timestamp':now,'product_base':BASE,'test_head':TEST,'document_parent':PARENT,'source_sha256':sources,'conflict_resolution':'Source-bound proposal, Owner pending; no core changes or dispatch'}
    dump(PACKET/'evidence/SOURCE_BINDINGS.json',source)
    bindings=[];native=[];metadata=[]
    for code,to,name in [('DSHA','DSH-A','02_DSHA_REVIEW_REQUEST.md'),('DSHB','DSH-B','03_DSHB_REVIEW_REQUEST.md')]:
        objective=(PACKET/'handoff'/name).read_text()
        acceptance=['核对材料SHA与本地来源；不把格式验证视为批准','登记Owner待批与INV-07b证据等级，保留原RED','不新增Worker/队列/核心实现或扩大现有授权']
        tid='ARCH-5915-'+code
        bindings.append({'task_id':tid,'recipient':to,'message_kind':'ARCHITECTURE_REVIEW_NOTICE_NOT_EXECUTION_TASK','objective':objective,'acceptance':acceptance,'objective_digest':digest(objective),'acceptance_digest':digest('\n'.join(acceptance)),'digest_canonicalization':'UTF-8 objective exact bytes; acceptance joined by LF with no final LF','owner_approval_refs':[],'owner_gate':True,'worker_dispatch_requested':False,'core_implementation_authorized':False})
        native.append({'handoff_id':'RUNTIME-5915-'+code,'task_id':tid,'from_harness':'codex-desktop','to_harness':to,'summary':'统一运行时架构裁决材料：INV-07b仅正常flush/reopen，无进程崩溃或WAL恢复证明；本轮4FAIL/2PASS。ADR-011及迁移/验收待Owner批准，仅请求材料登记，不请求派发。','artifacts':[{'kind':'decision_doc','ref':str((PACKET/'ADR-410-011-UNIFIED_RUNTIME_CONTRACT.md').relative_to(ROOT))},{'kind':'snapshot','ref':str((PACKET/'handoff/MANIFEST.json').relative_to(ROOT))}],'decisions':['C路线维持；此具体统一协议仍PROPOSED','只交付材料，不授权Worker/核心实现','INV-07b crash/WAL replay/power loss均NOT_RUN'],'open_questions':['持久RowId/格式','workspace预算','durability模式/故障模型','隔离/产品范围','迁移/回退与独立验收'],'context_fingerprint':sha(PACKET/'evidence/SOURCE_BINDINGS.json'),'created_at':now})
        metadata.append({'task_id':tid,'project':'sqlrustgo','objective_digest':digest(objective),'acceptance_count':len(acceptance),'acceptance_digest':digest('\n'.join(acceptance)),'allowed_paths_glob':[str(PACKET.relative_to(ROOT))+'/'],'forbidden_paths_glob':['src/','crates/','Cargo.toml','scripts/gate/','docs/governance/','.gitea/'],'risk':'L4','file_count':0,'line_stats':{'added':0,'removed':0},'base_sha':BASE,'execution_mode':'LOCAL_ONLY','budget_seconds':1,'budget_attempts':1})
    dump(PACKET/'handoff/01_LOCAL_BINDING.json',{'version':'RUNTIME-5915-v1','message_kind':'ARCHITECTURE_REVIEW_NOTICE','owner_approval':'PENDING','dispatch_requested':False,'core_implementation_authorized':False,'source_commit':BASE,'test_commit':TEST,'document_parent':PARENT,'bindings':bindings,'metadata_metrics':'file_count/line_stats cover MANIFEST content files, not core implementation scope or a dispatch permit'})
    (PACKET/'handoff/native_handoff.jsonl').write_text('\n'.join(json.dumps(x,ensure_ascii=False) for x in native)+'\n')
    meta_path=PACKET/'handoff/dsh_metadata.jsonl'
    meta_path.write_text('\n'.join(json.dumps(x,ensure_ascii=False,indent=None) for x in metadata)+'\n')
    files=content_files();line_count=sum(len(p.read_bytes().splitlines()) for p in files)
    for d in metadata:d['file_count']=len(files);d['line_stats']={'added':line_count,'removed':0}
    meta_path.write_text('\n'.join(json.dumps(x,ensure_ascii=False) for x in metadata)+'\n')
    files=content_files()
    manifest={'version':'RUNTIME-5915-v1','source_agent':'Codex GPT-6.1 Sol','source_run':'RUNTIME-5915-20261120','timestamp':now,'repository':'http://192.168.0.252:3000/openclaw/sqlrustgo.git','product_base':BASE,'test_head':TEST,'document_parent':PARENT,'status':'PROPOSED_OWNER_APPROVAL_PENDING','owner_approval_refs':[],'worker_dispatch_requested':False,'core_implementation_authorized':False,'adr_sha256':sha(PACKET/'ADR-410-011-UNIFIED_RUNTIME_CONTRACT.md'),'conflict_resolution':'Delivery is not authorization; existing DSH scopes unchanged','files':{str(p.relative_to(PACKET)):sha(p) for p in files},'file_count':len(files),'self_hash':'External SHA256, no self-reference','exclusions':sorted(SKIP)}
    dump(PACKET/'handoff/MANIFEST.json',manifest)
    sums=[h+'  '+p for p,h in manifest['files'].items()]
    sums.append(sha(PACKET/'handoff/MANIFEST.json')+'  handoff/MANIFEST.json')
    (PACKET/'handoff/SHA256SUMS.txt').write_text('\n'.join(sums)+'\n')

def validate_bindings(binding,matrix):
    assert binding['dispatch_requested'] is False and binding['core_implementation_authorized'] is False
    assert binding['owner_approval']=='PENDING'
    for b in binding['bindings']:
        assert b['owner_approval_refs']==[] and b['owner_gate']
        assert b['worker_dispatch_requested'] is False and b['core_implementation_authorized'] is False
    b=next(c for c in matrix['baseline_cases'] if c['id']=='PR5915-INV07b')
    assert b['fault']=='CLEAN_OBJECT_DROP_AND_REOPEN'
    assert b['process_crash_observed'] is False and b['wal_replay_observed'] is False and b['power_loss_observed'] is False
    assert matrix['baseline_summary']=={'tests_run':6,'passed':2,'failed':4,'ignored':0,'cargo_exit':101,'detector_execution_is_not_product_PASS':True}

def check():
    from jsonschema import Draft202012Validator
    manifest=read(PACKET/'handoff/MANIFEST.json')
    for p,h in manifest['files'].items():assert sha(PACKET/p)==h,p
    assert manifest['adr_sha256']==sha(PACKET/'ADR-410-011-UNIFIED_RUNTIME_CONTRACT.md')
    binding=read(PACKET/'handoff/01_LOCAL_BINDING.json');matrix=read(PACKET/'04_COUNTEREXAMPLE_ACCEPTANCE_MATRIX.json')
    validate_bindings(binding,matrix)
    module=reviewed_handoff()
    metadata=[json.loads(l) for l in (PACKET/'handoff/dsh_metadata.jsonl').read_text().splitlines()]
    native=[json.loads(l) for l in (PACKET/'handoff/native_handoff.jsonl').read_text().splitlines()]
    schema=Draft202012Validator(read(PACKET/'evidence/inputs/dsh-handoff.schema.json'))
    assert len(metadata)==len(native)==len(binding['bindings'])==2
    for d,b,n in zip(metadata,binding['bindings'],native):
        assert set(d)==set(module.ALLOWED_FIELDS) and d['execution_mode']=='LOCAL_ONLY' and d['risk']=='L4'
        assert d['file_count']==manifest['file_count']
        assert d['line_stats']=={'added':sum(len((PACKET/p).read_bytes().splitlines()) for p in manifest['files']),'removed':0}
        assert d['objective_digest']==digest(b['objective']) and d['acceptance_digest']==digest('\n'.join(b['acceptance']))
        assert d['acceptance_count']==len(b['acceptance'])
        assert n['context_fingerprint']==sha(PACKET/'evidence/SOURCE_BINDINGS.json')
        module.validate(d,strict=True);schema.validate(n)
    rejected=[]
    for flag in ['dispatch_requested','core_implementation_authorized']:
        bad=copy.deepcopy(binding);bad[flag]=True
        try:validate_bindings(bad,matrix)
        except AssertionError:rejected.append(flag)
        else:raise AssertionError('accepted unauthorized mutation')
    bad=copy.deepcopy(matrix);next(c for c in bad['baseline_cases'] if c['id']=='PR5915-INV07b')['process_crash_observed']=True
    try:validate_bindings(binding,bad)
    except AssertionError:rejected.append('false_crash_claim')
    else:raise AssertionError('accepted false crash claim')
    bad=dict(metadata[0]);bad['source_code']='forbidden'
    try:module.validate(bad,strict=True)
    except module.HandoffViolation:rejected.append('controller_source_payload')
    else:raise AssertionError('accepted source payload')
    run=read(PACKET/'evidence/independent-pr5915.json');raw=read(PACKET/'evidence/independent-pr5915.raw.json')
    raw_bytes=base64.b64decode(raw['data_base64'],validate=True);log=raw_bytes.decode()
    assert run['raw_log']==raw['logical_name'] and raw['raw_sha256']==hashlib.sha256(raw_bytes).hexdigest()
    assert run['exit_code']==101 and run['evidence_hash']=='sha256:'+hashlib.sha256(raw_bytes).hexdigest()
    assert log.count('1 passed; 2 failed; 0 ignored')==2
    assert 'test inv01b_mutation_revoked_exactly_once ... ok' in log
    assert 'test inv07c_rollback_then_reopen_is_clean ... ok' in log
    inputs=PACKET/'evidence/inputs'
    for name in ['ca01_file_undo.rs','ca01_snapshot_reopen.rs']:
        assert (inputs/name).read_bytes()==subprocess.check_output(['git','show',TEST+':tests/integration/test_factory/'+name],cwd=ROOT)
    old=ROOT/'docs/refactor/v4.1.0-c/handoff/10_HANDOFF_MANIFEST.json'
    assert sha(old)=='c722885fef98a93ec0eae8157104861fffdeaa1454bf4710bd12464c25ab76de'
    for p,h in read(old)['files'].items():assert sha(ROOT/p)==h,p
    changed=subprocess.check_output(['git','diff',PARENT,'--name-only'],cwd=ROOT,text=True).splitlines()
    assert all(p.startswith(str(PACKET.relative_to(ROOT))+'/') for p in changed),changed
    for p in PACKET.rglob('*.md'):
        for l in re.findall(r'\]\(([^)]+)\)',p.read_text()):
            if not re.match(r'[a-z]+://',l):assert (p.parent/l.split('#')[0]).exists(),(p,l)
    return {'source_agent':'Codex GPT-6.1 Sol','source_run':'RUNTIME-5915-20261120','timestamp':datetime.datetime.now(datetime.timezone.utc).isoformat(),'document_validation_only':True,'bound_files':manifest['file_count'],'native_handoffs':len(native),'strict14_envelopes':len(metadata),'baseline_tests':6,'product_failed':4,'product_passed':2,'inv07b_crash_evidence':'NOT_RUN','negative_checks_rejected':rejected,'old_manifest_unchanged':True,'core_changes':0,'workers_dispatched':0,'owner_approval':'PENDING','manifest_sha256':sha(PACKET/'handoff/MANIFEST.json'),'adr_sha256':manifest['adr_sha256']}

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--finalize',action='store_true');p.add_argument('--check',action='store_true');args=p.parse_args()
    if args.finalize:build()
    if args.check:print(json.dumps(check(),ensure_ascii=False,indent=2))
