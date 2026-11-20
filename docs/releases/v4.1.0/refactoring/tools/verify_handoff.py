#!/usr/bin/env python3
"""Offline artifact/schema validator. No queue, model, agent or service calls."""
import argparse
import copy
import datetime
import hashlib
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[5]
PLAN = ROOT / "docs/releases/v4.1.0/refactoring"
HAND = ROOT / "docs/refactor/v4.1.0-c/handoff"
BASE = "e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0"
DSH = pathlib.Path("/Users/liying/dev/workspaces/deepseek-schedule/deepseek-schedule")
KOS = pathlib.Path("/Users/liying/dev/workspaces/kos/engineering-os-control")

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def read(path):
    return json.loads(path.read_text())

def dump(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data,ensure_ascii=False,indent=2)+"\n")

def git(cwd,*args):
    return subprocess.check_output(["git",*args],cwd=cwd,text=True).strip()

def acyclic(tasks, gates):
    ids={x["task_id"] for x in tasks}
    assert len(ids)==len(tasks), "duplicate task id"
    seen,active=set(),set()
    mapping={t["task_id"]:t for t in tasks}
    def visit(n):
        assert n not in active, "dependency cycle: "+n
        if n in seen or n in gates:
            return
        assert n in mapping,"missing dependency: "+n
        active.add(n)
        for dep in mapping[n]["dependencies"]:visit(dep)
        active.remove(n);seen.add(n)
    for n in ids:visit(n)

def validate(dag,native):
    from jsonschema import Draft202012Validator
    validator=Draft202012Validator(read(PLAN/"evidence/contracts/dsh-task.schema.json"))
    for t in native:validator.validate(t)
    assert len(native)==18
    assert {t["task_id"] for t in native}=={t["task_id"] for t in dag["tasks"]}
    native_by_id={t["task_id"]:t for t in native}
    gates={g["id"] for g in dag["external_gates"]}
    acyclic(dag["tasks"],gates)
    for t in dag["tasks"]:
        assert t["milestone_id"]=="C-ALPHA-01"
        assert t["model_policy"]=="LOCAL_ONLY"
        assert t["approved_adr_refs"]==[]
        assert t["authorized_for_execution"] is False and t["dispatch_allowed"] is False
        assert set(t["human_gate"]["gates"])<=gates
        assert t["required_tests"] and t["stop_conditions"] and t["acceptance_criteria"]
        assert t["review_requirements"]["independent_harness"]
        assert t["base_sha"]==BASE
        n=native_by_id[t["task_id"]]
        assert n["base_commit"]==t["base_sha"] and n["risk"]==t["risk_level"]
        assert n["allowed_paths"]==t["allowed_paths"] and n["forbidden_paths"]==t["forbidden_paths"]
        assert t["token_budget"]["status"]=="PROPOSED_NOT_AUTHORIZED"
        assert t["token_budget"]["upper_all_attempts"]>0
        assert t["retry_limit"]<=2
        if t["risk_level"]=="L4":assert t["human_gate"]["required"]
    assert all(x["dispatch_allowed"] is False for x in dag["future_epics"])
    assert dag["budget"]["authorized_tokens"]==0
    assert sum(x["token_budget"]["upper_all_attempts"] for x in dag["tasks"])==2570000

def load_tasks():
    return read(PLAN/"REFACTOR_TASK_DAG.json"),[json.loads(l) for l in (HAND/"03_TASK_MANIFEST.jsonl").read_text().splitlines() if l.strip()]

def finalize():
    sources={}
    names=["contracts/task.schema.json","contracts/result.schema.json","contracts/receipt.schema.json","contracts/handoff.schema.json","scheduler/router.py","scheduler/gatekeeper.py","scheduler/state_machine.py","adapters/opencode.py","adapters/omp_strata.py","config/routing.json","config/policies.json","docs/test-factory/TEST_CORPUS_MANIFEST.json","docs/test-factory/hold/LOCAL_ONLY_ATTESTATION.md","docs/test-factory/hold/LOCAL-ONLY-FAIL-CLOSED-TESTS.md","docs/test-factory/hold/EXECUTION-MODE-CONTRACT.md"]
    for n in names:sources[str(DSH/n)]=sha(DSH/n)
    for n in ["docs/EVIDENCE-CONTRACT.md","docs/STATE-TRUTH.md","orchestration/task_state.py"]:sources[str(KOS/n)]=sha(KOS/n)
    for p in [pathlib.Path("/Volumes/workspace/dev/srg-test-factory-pilot/tools/test_factory.py"),pathlib.Path("/Volumes/workspace/dev/srg-test-factory-pilot/tests/integration/test_factory/tf_t1_undo_invariants.rs")]:sources[str(p)]=sha(p)
    stamp=datetime.datetime.now(datetime.timezone.utc).isoformat()
    bindings={"source_agent":"Codex GPT-6.1 Sol","source_run":"C-ALPHA-01-HANDOFF-20261120","timestamp":stamp,"base_sha":BASE,"dsh_local_sha":git(DSH,"rev-parse","HEAD"),"kos_local_sha":git(KOS,"rev-parse","HEAD"),"remote_schema_heads_verified":False,"source_sha256":sources,"conflict_resolution":"Source-bound plan, no dispatch or independent certification"}
    dump(PLAN/"evidence/SOURCE_BINDINGS.json",bindings)
    adrs=[{"id":p.stem,"path":str(p.relative_to(ROOT)),"sha256":sha(p),"status":"PROPOSED","approval_refs":[]} for p in sorted((PLAN/"adrs").glob("*.md"))]
    dump(HAND/"01_ARCHITECTURE_BASELINE.json",{"version":"C-HANDOFF-1","base_sha":BASE,"tree":"4194fc975ebaf749de477f42ca5a300facef1278","source_run":bindings["source_run"],"timestamp":stamp,"stage":"ALPHA","hara_manifest":{"path":str((PLAN/"audit/HARA-410/evidence/MANIFEST.json").relative_to(ROOT)),"sha256":sha(PLAN/"audit/HARA-410/evidence/MANIFEST.json")},"source_bindings":{"path":str((PLAN/"evidence/SOURCE_BINDINGS.json").relative_to(ROOT)),"sha256":sha(PLAN/"evidence/SOURCE_BINDINGS.json")},"adrs":adrs,"architecture_principle":"OWNER_ACCEPTED_C","adr_approval":"PENDING","dispatch_allowed":False,"pending_owner_decisions":["DSH takeover","18-task budget","LOCAL_ONLY enforcement","ADR digests","isolation/product scope","format/sidecar","resource windows","independent reviewer"]})
    dump(HAND/"04_TASK_DEPENDENCY_DAG.json",{"kind":"HASH_BOUND_REFERENCE_NOT_SECOND_DAG","path":str((PLAN/"REFACTOR_TASK_DAG.json").relative_to(ROOT)),"sha256":sha(PLAN/"REFACTOR_TASK_DAG.json"),"dispatch_allowed":False})
    files={}
    for directory in [PLAN,HAND.parent]:
        for p in sorted(directory.rglob("*")):
            if p.is_file() and "__pycache__" not in p.parts and p.name!="10_HANDOFF_MANIFEST.json" and p.name not in {"validation.json","PUBLICATION_RECEIPT.json"}:
                files[str(p.relative_to(ROOT))]=sha(p)
    dump(HAND/"10_HANDOFF_MANIFEST.json",{"version":"C-HANDOFF-1","source_agent":"Codex GPT-6.1 Sol","source_run":bindings["source_run"],"timestamp":stamp,"base_sha":BASE,"source_uri":"http://192.168.0.252:3000/openclaw/sqlrustgo","conflict_resolution":"Owner approval and live DSH policy required","status":"CANDIDATE_NOT_AUTHORIZED","files":files,"file_count":len(files),"manifest_self_hash":"External sha256sum; excluded to avoid recursion","exclusions":["10_HANDOFF_MANIFEST.json","validation.json","PUBLICATION_RECEIPT.json"]})

def check():
    dag,native=load_tasks();validate(dag,native)
    rejected=[]
    mutations=[]
    d=copy.deepcopy(dag);d["tasks"][0]["dependencies"]=["C01-18"];mutations.append(("cycle",d,native))
    d=copy.deepcopy(dag);d["tasks"][0]["dependencies"]=["MISSING"];mutations.append(("missing",d,native))
    d=copy.deepcopy(dag);d["tasks"][0]["dispatch_allowed"]=True;mutations.append(("unapproved_dispatch",d,native))
    d=copy.deepcopy(dag);d["tasks"][0]["approved_adr_refs"]=["fake"];mutations.append(("unapproved_adr",d,native))
    d=copy.deepcopy(dag);d["tasks"][12]["human_gate"]["required"]=False;mutations.append(("human_gate_removed",d,native))
    d=copy.deepcopy(dag);d["tasks"][0]["model_policy"]="LOCAL_FIRST";mutations.append(("cloud_policy",d,native))
    d=copy.deepcopy(dag);d["tasks"][0]["milestone_id"]="C-ALPHA-02";mutations.append(("later_stage_task",d,native))
    n=copy.deepcopy(native);n[0]["model_policy"]="LOCAL_ONLY";mutations.append(("native_extra_field",dag,n))
    for name,d,n in mutations:
        try:validate(d,n)
        except Exception:rejected.append(name)
        else:raise AssertionError("mutation accepted: "+name)
    a=PLAN/"audit/HARA-410";m=read(a/"evidence/MANIFEST.json")
    assert sha(a/"evidence/MANIFEST.json")=="4649fccc566613f04568ef3564e9d585739c4e92c73ad5972714b7d4f65b992f"
    for p,h in m["files"].items():assert sha(a/p)==h,p
    manifest=read(HAND/"10_HANDOFF_MANIFEST.json")
    for p,h in manifest["files"].items():assert sha(ROOT/p)==h,p
    ref=read(HAND/"04_TASK_DEPENDENCY_DAG.json");assert sha(ROOT/ref["path"])==ref["sha256"]
    baseline=read(HAND/"01_ARCHITECTURE_BASELINE.json")
    assert len(baseline["adrs"])==10
    for d in baseline["adrs"]:assert sha(ROOT/d["path"])==d["sha256"] and d["approval_refs"]==[]
    assert len(read(PLAN/"TEST_COVERAGE_MATRIX.json")["cases"])==75
    links=0
    for directory in [PLAN,HAND.parent]:
        for p in directory.rglob("*.md"):
            for link in re.findall(r"\]\(([^)]+)\)",p.read_text()):
                if not re.match(r"[a-z]+://",link):
                    assert (p.parent/link.split("#")[0]).exists(),str(p)+":"+link
                    links+=1
    changed=git(ROOT,"diff","HEAD","--name-only").splitlines()
    assert all(p.startswith(("docs/releases/v4.1.0/refactoring/","docs/refactor/v4.1.0-c/")) for p in changed),changed
    return {"kind":"OFFLINE_DELIVERY_VALIDATION_NOT_EXECUTION_CERTIFICATION","source_run":"C-ALPHA-01-HANDOFF-20261120","timestamp":datetime.datetime.now(datetime.timezone.utc).isoformat(),"native_tasks":18,"adrs":10,"pilot_input_cases":75,"hara_hashed_files":55,"manifest_bound_files":len(manifest["files"]),"task_dag":"ACYCLIC_NO_MISSING_DEPENDENCIES","mutation_rejections":rejected,"local_links":links,"authorized_dispatch_count":0,"authorized_tokens":0,"proposed_token_upper":2570000,"handoff_manifest_sha256":sha(HAND/"10_HANDOFF_MANIFEST.json")}

if __name__=="__main__":
    parser=argparse.ArgumentParser();parser.add_argument("--finalize",action="store_true");parser.add_argument("--check",action="store_true");args=parser.parse_args()
    if args.finalize:finalize()
    if args.check:
        result=check();print(json.dumps(result,ensure_ascii=False,indent=2))
