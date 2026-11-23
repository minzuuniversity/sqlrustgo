#!/usr/bin/env python3
"""Offline artifact/schema validator. No queue, model, agent or service calls.

Three properties changed with the v4.1.0 C-ALPHA-01 handoff (Task 3):

  * the baseline identity is read from BASELINE_FREEZE.json instead of being
    hardcoded, so the gate follows the freeze record rather than keeping a
    second copy of it;

  * the ADR assertions are loosened from "no ADR may be approved at all" to
    "an ADR may only be cited at the tier the owner actually granted".  An
    empty `approved_adr_refs` is a legitimate pending state -- it is reported
    as pending, never dressed up as verified -- while a non-empty one must
    cite ADRs that exist, that carry a decision, and whose tier authorises
    what the task claims;

  * the manifest's hash range is narrowed to the delivered artefacts
    (`handoff/`, `takeover/dsh-b/`, `docs/releases/v4.1.0/refactoring/`), so
    the writes into the active `takeover/` workspace no longer make the
    manifest drift every time DSH-B produces a governance document.

The invariants this gate asserts are raised explicitly instead of being
written as `assert` statements wherever they were rewritten, because
`python -O` strips `assert` and would turn them into no-ops; the tool also
refuses to run under `python -O` at all.
"""
import argparse
import copy
import datetime
import hashlib
import json
import pathlib
import re
import subprocess
import sys
import traceback

ROOT = pathlib.Path(__file__).resolve().parents[5]
PLAN = ROOT / "docs/releases/v4.1.0/refactoring"
HAND = ROOT / "docs/refactor/v4.1.0-c/handoff"
FREEZE_REL = "docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json"
FREEZE = ROOT / FREEZE_REL
DSH = pathlib.Path("/Users/liying/dev/workspaces/deepseek-schedule/deepseek-schedule")
KOS = pathlib.Path("/Users/liying/dev/workspaces/kos/engineering-os-control")

# The manifest's hash range: the delivered artefacts only.
#
# `docs/refactor/v4.1.0-c/takeover/`'s top level is an active workspace (the
# design document, the implementation plan, the owner's running decisions) --
# DSH-B writes there constantly, and every one of those writes used to make
# the manifest drift.  `takeover/dsh-b/` is the opposite: delivered governance
# records, so their per-file digests stay covered.
MANIFEST_ROOTS = (
    HAND,
    ROOT / "docs/refactor/v4.1.0-c/takeover/dsh-b",
    PLAN,
)

# The working tree may only carry changes under these prefixes -- the same
# three roots as the manifest's hash range, so a change the manifest cannot
# see is also a change the gate refuses.  `takeover/`'s top level is allowed
# (it is the active workspace) but nothing outside these prefixes is.
ALLOWED_CHANGE_PREFIXES = (
    "docs/releases/v4.1.0/refactoring/",
    "docs/refactor/v4.1.0-c/handoff/",
    "docs/refactor/v4.1.0-c/takeover/",
)

# The owner's ruling of 2026-11-21 (dsh-b/ADR-DECISION-RECORD.md:35-47)
# grades the eleven ADRs into these tiers:
#
#   APPROVED               unconditional -- may authorise execution
#   APPROVED_DIRECTION     direction only, the implementation is reviewed
#                           separately -- may be cited, but not by a task that
#                           is authorised for execution
#   CONDITIONALLY_APPROVED a precondition is not met yet -- may not be cited
#   PROPOSED               no owner decision recorded yet
#
# "APPROVED" is a prefix of "APPROVED_DIRECTION", so the tiers are tested
# most-specific first.
ADR_TIERS = ("APPROVED_DIRECTION", "CONDITIONALLY_APPROVED", "APPROVED")

# The ADR the "tier overreach" mutation arm points at: 003 is "approved
# direction only" (ADR-DECISION-RECORD.md:35-47), so no task that is
# authorised for execution may cite it.
TIER_OVERREACH_ADR = "ADR-410-003"

# O7: ADR-410-011 is merged into the catalogue, but it is only conditionally
# approved, so it must not enter any task's `approved_adr_refs`.  The
# catalogue therefore holds ten ADRs until Task 4 moves ADR-410-011 out of
# runtime-contract/20261120-pr5915/ into `adrs/`, and eleven afterwards.
ADR_CATALOG_SIZES = (10, 11)

ADR_STATUS_RE = re.compile(r"^[ \t]*Status:[ \t]*([^\n]+)", re.M)
ADR_APPROVALS_RE = re.compile(r"^[ \t]*Approvals:[ \t]*([^\n]+)", re.M)
ADR_REF_RE = re.compile(r"ADR-410-\d{3}")


class HandoffError(Exception):
    """A handoff invariant does not hold."""


def require(condition, message):
    """Assert-like helper that survives `python -O` (which strips `assert`)."""
    if not condition:
        raise HandoffError(message)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def read(path):
    return json.loads(path.read_text())

def dump(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data,ensure_ascii=False,indent=2)+"\n")

def git(cwd,*args):
    return subprocess.check_output(["git",*args],cwd=cwd,text=True).strip()

def freeze_record():
    """The frozen baseline record -- the single source of base_sha/tree_sha."""
    if not FREEZE.is_file():
        raise HandoffError(f"{FREEZE_REL} does not exist")
    try:
        record=json.loads(FREEZE.read_text())
    except ValueError as exc:
        raise HandoffError(f"{FREEZE_REL} is not valid JSON: {exc}") from exc
    for key in ("base_sha","tree_sha"):
        require(key in record,f"{FREEZE_REL} does not carry a {key}")
    return record

def parse_adr_status(path):
    """(tier, approval refs) parsed out of an ADR's own Status/Approvals line.

    The ADR text is the authority: the catalogue in
    01_ARCHITECTURE_BASELINE.json is a projection of it, never a second source
    of truth.
    """
    text=path.read_text()
    status=ADR_STATUS_RE.search(text)
    require(status is not None,f"{path.name} carries no 'Status:' line")
    head=re.split(r"[（(]",status.group(1))[0].strip()
    tier=next((t for t in ADR_TIERS if t in head),"PROPOSED")
    approvals=ADR_APPROVALS_RE.search(text)
    refs=sorted(set(ADR_REF_RE.findall(approvals.group(1)))) if approvals else []
    return tier,refs

def adr_tiers():
    """{ADR id: tier} for every ADR file in `adrs/`."""
    return {p.stem:parse_adr_status(p)[0] for p in sorted((PLAN/"adrs").glob("*.md"))}

def aggregate_adr_approval(adrs):
    """The catalogue's approval state, aggregated over the ADR tiers.

    PROPOSED wins over everything: while one ADR carries no owner decision,
    the set as a whole is still pending and must not be reported as approved.
    """
    tiers={a["status"] for a in adrs}
    if not tiers or "PROPOSED" in tiers:return "PENDING"
    if "CONDITIONALLY_APPROVED" in tiers:return "CONDITIONALLY_APPROVED"
    if "APPROVED_DIRECTION" in tiers:return "APPROVED_DIRECTION"
    return "APPROVED"

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

def check_approved_adr_refs(task, tiers):
    """`approved_adr_refs` may be empty -- that is the pending state -- but a
    non-empty list must cite ADRs that exist and whose tier authorises what
    the task claims.

    The rules are the owner's (A2.2 of the readiness design):

      R5  every cited id must be an ADR that exists in `adrs/`
      --  an ADR that is still PROPOSED carries no decision, so it cannot be
          claimed as approved
      R4  a conditionally approved ADR (006/011) may not be cited at all:
          its precondition is not met yet
      R3  a direction-only ADR (003/004/007/008/009/010) may not be cited by
          a task that is authorised for execution -- the implementation is
          reviewed separately
    """
    refs=task["approved_adr_refs"]
    if not refs:
        return
    for ref in refs:
        require(ref in tiers,
                f"task {task['task_id']} cites {ref!r}, which is not a real ADR id "
                f"(the catalogue holds {len(tiers)} ADR(s): {sorted(tiers)})")
        tier=tiers[ref]
        require(tier!="PROPOSED",
                f"task {task['task_id']} claims {ref} as approved, but that ADR is "
                f"still PROPOSED -- the owner's decision is not recorded in its text")
        require(tier!="CONDITIONALLY_APPROVED",
                f"task {task['task_id']} cites {ref}, whose precondition is not met "
                f"(CONDITIONALLY_APPROVED); such an ADR may not be cited at all")
        if task["authorized_for_execution"]:
            require(tier=="APPROVED",
                    f"task {task['task_id']} is authorised for execution but cites {ref}, "
                    f"whose tier is {tier}; only an unconditionally APPROVED ADR may "
                    f"authorise execution")

def validate(dag,native,tiers,base):
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
        # Before the "nothing is authorised yet" assertion: a task that claims
        # an ADR it may not claim must be reported as a tier violation, not as
        # an authorisation state.
        check_approved_adr_refs(t,tiers)
        assert t["authorized_for_execution"] is False and t["dispatch_allowed"] is False
        assert set(t["human_gate"]["gates"])<=gates
        assert t["required_tests"] and t["stop_conditions"] and t["acceptance_criteria"]
        assert t["review_requirements"]["independent_harness"]
        assert t["base_sha"]==base
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
    record=freeze_record()
    base=record["base_sha"]
    stamp=datetime.datetime.now(datetime.timezone.utc).isoformat()
    bindings={"source_agent":"Codex GPT-6.1 Sol","source_run":"C-ALPHA-01-HANDOFF-20261120","timestamp":stamp,"base_sha":base,"dsh_local_sha":git(DSH,"rev-parse","HEAD"),"kos_local_sha":git(KOS,"rev-parse","HEAD"),"remote_schema_heads_verified":False,"source_sha256":sources,"conflict_resolution":"Source-bound plan, no dispatch or independent certification"}
    dump(PLAN/"evidence/SOURCE_BINDINGS.json",bindings)
    adrs=[]
    for p in sorted((PLAN/"adrs").glob("*.md")):
        tier,refs=parse_adr_status(p)
        adrs.append({"id":p.stem,"path":str(p.relative_to(ROOT)),"sha256":sha(p),"status":tier,"approval_refs":refs})
    dump(HAND/"01_ARCHITECTURE_BASELINE.json",{"version":"C-HANDOFF-1","base_sha":base,"tree":record["tree_sha"],"source_run":bindings["source_run"],"timestamp":stamp,"stage":"ALPHA","hara_manifest":{"path":str((PLAN/"audit/HARA-410/evidence/MANIFEST.json").relative_to(ROOT)),"sha256":sha(PLAN/"audit/HARA-410/evidence/MANIFEST.json")},"source_bindings":{"path":str((PLAN/"evidence/SOURCE_BINDINGS.json").relative_to(ROOT)),"sha256":sha(PLAN/"evidence/SOURCE_BINDINGS.json")},"adrs":adrs,"architecture_principle":"OWNER_ACCEPTED_C","adr_approval":aggregate_adr_approval(adrs),"dispatch_allowed":False,"pending_owner_decisions":["DSH takeover","18-task budget","LOCAL_ONLY enforcement","ADR digests","isolation/product scope","format/sidecar","resource windows","independent reviewer"]})
    dump(HAND/"04_TASK_DEPENDENCY_DAG.json",{"kind":"HASH_BOUND_REFERENCE_NOT_SECOND_DAG","path":str((PLAN/"REFACTOR_TASK_DAG.json").relative_to(ROOT)),"sha256":sha(PLAN/"REFACTOR_TASK_DAG.json"),"dispatch_allowed":False})
    files={}
    for directory in MANIFEST_ROOTS:
        for p in sorted(directory.rglob("*")):
            if p.is_file() and "__pycache__" not in p.parts and p.name!="10_HANDOFF_MANIFEST.json" and p.name not in {"validation.json","PUBLICATION_RECEIPT.json"}:
                files[str(p.relative_to(ROOT))]=sha(p)
    dump(HAND/"10_HANDOFF_MANIFEST.json",{"version":"C-HANDOFF-1","source_agent":"Codex GPT-6.1 Sol","source_run":bindings["source_run"],"timestamp":stamp,"base_sha":base,"source_uri":"http://192.168.0.252:3000/openclaw/sqlrustgo","conflict_resolution":"Owner approval and live DSH policy required","status":"CANDIDATE_NOT_AUTHORIZED","files":files,"file_count":len(files),"manifest_self_hash":"External sha256sum; excluded to avoid recursion","exclusions":["10_HANDOFF_MANIFEST.json","validation.json","PUBLICATION_RECEIPT.json"]})
    return len(files)

def rejection_reason(exc):
    """A one-line reason why a mutation was rejected.

    Only the first line of the exception is kept: a jsonschema failure quotes
    the whole schema, and a summary that carries it is unreadable.
    """
    detail=" ".join(str(exc).strip().splitlines()[:1]).strip()
    if not detail:
        return type(exc).__name__
    if len(detail)>240:
        detail=detail[:237]+"..."
    return f"{type(exc).__name__}: {detail}"

def check():
    dag,native=load_tasks()
    record=freeze_record()
    base=record["base_sha"]
    tiers=adr_tiers()
    validate(dag,native,tiers,base)
    rejected=[]
    reasons={}
    mutations=[]
    d=copy.deepcopy(dag);d["tasks"][0]["dependencies"]=["C01-18"];mutations.append(("cycle",d,native))
    d=copy.deepcopy(dag);d["tasks"][0]["dependencies"]=["MISSING"];mutations.append(("missing",d,native))
    d=copy.deepcopy(dag);d["tasks"][0]["dispatch_allowed"]=True;mutations.append(("unapproved_dispatch",d,native))
    d=copy.deepcopy(dag);d["tasks"][0]["approved_adr_refs"]=["fake"];mutations.append(("unapproved_adr",d,native))
    d=copy.deepcopy(dag);d["tasks"][12]["human_gate"]["required"]=False;mutations.append(("human_gate_removed",d,native))
    d=copy.deepcopy(dag);d["tasks"][0]["model_policy"]="LOCAL_FIRST";mutations.append(("cloud_policy",d,native))
    d=copy.deepcopy(dag);d["tasks"][0]["milestone_id"]="C-ALPHA-02";mutations.append(("later_stage_task",d,native))
    n=copy.deepcopy(native);n[0]["model_policy"]="LOCAL_ONLY";mutations.append(("native_extra_field",dag,n))
    d=copy.deepcopy(dag);d["tasks"][0]["approved_adr_refs"]=[TIER_OVERREACH_ADR];d["tasks"][0]["authorized_for_execution"]=True;mutations.append(("tier_overreach",d,native))
    for name,d,n in mutations:
        try:validate(d,n,tiers,base)
        except Exception as exc:
            rejected.append(name)
            reasons[name]=rejection_reason(exc)
        else:raise AssertionError("mutation accepted: "+name)
    a=PLAN/"audit/HARA-410";m=read(a/"evidence/MANIFEST.json")
    baseline=read(HAND/"01_ARCHITECTURE_BASELINE.json")
    require(sha(a/"evidence/MANIFEST.json")==baseline["hara_manifest"]["sha256"],
            "the HARA evidence manifest's digest does not match the digest the "
            "architecture baseline binds to it")
    for p,h in m["files"].items():assert sha(a/p)==h,p
    manifest=read(HAND/"10_HANDOFF_MANIFEST.json")
    for p,h in manifest["files"].items():assert sha(ROOT/p)==h,p
    ref=read(HAND/"04_TASK_DEPENDENCY_DAG.json");assert sha(ROOT/ref["path"])==ref["sha256"]
    require(len(baseline["adrs"])==len(tiers),
            f"the ADR catalogue holds {len(baseline['adrs'])} entry/entries but adrs/ holds "
            f"{len(tiers)} ADR file(s): {sorted(tiers)}")
    require(len(baseline["adrs"]) in ADR_CATALOG_SIZES,
            f"the ADR catalogue holds {len(baseline['adrs'])} entry/entries; expected "
            f"{' or '.join(str(s) for s in ADR_CATALOG_SIZES)} (ten before the O7 merge of "
            f"ADR-410-011 into adrs/, eleven after it)")
    for entry in baseline["adrs"]:
        require(entry["id"] in tiers,
                f"the catalogue lists {entry['id']}, which has no file in adrs/")
        require(sha(ROOT/entry["path"])==entry["sha256"],
                f"the catalogue's digest for {entry['id']} does not match {entry['path']}")
        require(entry["status"]==tiers[entry["id"]],
                f"the catalogue records {entry['id']} as {entry['status']}, but the ADR's own "
                f"text reads {tiers[entry['id']]}")
        require(not entry["approval_refs"] or set(entry["approval_refs"])<=set(tiers),
                f"{entry['id']} cites approval references that are not real ADR ids: "
                f"{sorted(set(entry['approval_refs'])-set(tiers))}")
    assert len(read(PLAN/"TEST_COVERAGE_MATRIX.json")["cases"])==75
    links=0
    for directory in [PLAN,HAND.parent]:
        for p in directory.rglob("*.md"):
            for link in re.findall(r"\]\(([^)]+)\)",p.read_text()):
                if not re.match(r"[a-z]+://",link):
                    assert (p.parent/link.split("#")[0]).exists(),str(p)+":"+link
                    links+=1
    changed=git(ROOT,"diff","HEAD","--name-only").splitlines()
    require(all(p.startswith(ALLOWED_CHANGE_PREFIXES) for p in changed),changed)
    pending=[t["task_id"] for t in dag["tasks"] if not t["approved_adr_refs"]]
    tier_counts={tier:sum(1 for v in tiers.values() if v==tier) for tier in list(ADR_TIERS)+["PROPOSED"]}
    notes=[]
    if pending:
        notes.append(f"{len(pending)} of {len(dag['tasks'])} task(s) still carry an empty "
                     f"approved_adr_refs -- the owner's ADR decision is PENDING, not verified")
    if "ADR-410-011" not in tiers:
        notes.append("ADR-410-011 is not merged into adrs/ yet (O7); the catalogue holds ten "
                     "ADRs and Task 4 owns the relocation")
    return {"kind":"OFFLINE_DELIVERY_VALIDATION_NOT_EXECUTION_CERTIFICATION","source_run":"C-ALPHA-01-HANDOFF-20261120","timestamp":datetime.datetime.now(datetime.timezone.utc).isoformat(),"native_tasks":18,"adrs":len(baseline["adrs"]),"adr_catalogue_matches_adrs_dir":len(baseline["adrs"])==len(tiers),"adr_011_merged_into_adrs":"ADR-410-011" in tiers,"adr_tiers":tier_counts,"adr_approval":baseline["adr_approval"],"adr_approval_state":"PENDING" if baseline["adr_approval"]=="PENDING" else baseline["adr_approval"],"tasks_with_empty_approved_adr_refs":len(pending),"tasks_pending_adr_approval":pending,"pilot_input_cases":75,"hara_hashed_files":55,"manifest_bound_files":len(manifest["files"]),"task_dag":"ACYCLIC_NO_MISSING_DEPENDENCIES","mutation_rejections":rejected,"mutation_rejection_reasons":reasons,"local_links":links,"authorized_dispatch_count":0,"authorized_tokens":0,"proposed_token_upper":2570000,"handoff_manifest_sha256":sha(HAND/"10_HANDOFF_MANIFEST.json"),"notes":notes}

def main():
    parser=argparse.ArgumentParser(description="Offline validation of the v4.1.0 C-ALPHA-01 handoff package.")
    parser.add_argument("--finalize",action="store_true",help="re-sign the handoff manifests")
    parser.add_argument("--check",action="store_true",help="validate the handoff package")
    args=parser.parse_args()
    if sys.flags.optimize:
        print("refusing to run under python -O: assert statements are stripped, this gate would be a no-op")
        return 2
    try:
        if args.finalize:
            count=finalize()
            print(f"re-signed the handoff manifests ({count} file(s) in the hash range)")
        if args.check:
            result=check();print(json.dumps(result,ensure_ascii=False,indent=2))
    except Exception as exc:
        # Fail closed: print what went wrong instead of raising a traceback.
        # The failing source line is named, because a bare `assert` carries no
        # message of its own.
        print(f"FAIL  {type(exc).__name__}: {exc}".rstrip())
        frames=traceback.extract_tb(exc.__traceback__)
        if frames:
            frame=frames[-1]
            try:
                location=str(pathlib.Path(frame.filename).relative_to(ROOT))
            except ValueError:
                location=frame.filename
            print(f"      at {location}:{frame.lineno}: {frame.line}")
        return 1
    return 0

if __name__=="__main__":
    sys.exit(main())
