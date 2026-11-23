#!/usr/bin/env python3
"""Offline artifact/schema validator. No queue, model, agent or service calls.

What this gate asserts
----------------------
The v4.1.0 C-ALPHA-01 handoff package is a set of files that claim to be a
ready-to-execute plan.  This gate checks the claims against each other:

  * the baseline identity comes from one place -- `BASELINE_FREEZE.json` --
    and the freeze record itself is covered by the manifest, so the file the
    identity rests on cannot drift away from the package that cites it;

  * the HARA evidence manifest is pinned to the frozen base commit's copy of
    it.  That anchor cannot be rewritten by `--finalize`, so at least one
    digest in this tool is not re-derivable from the artefact it checks;

  * an ADR may only be cited at the tier the owner actually granted.  An
    empty `approved_adr_refs` is a legitimate pending state -- reported as
    pending, never dressed up as verified -- while a non-empty one must cite
    ADRs that exist, that carry a decision, and whose tier authorises what
    the task claims;

  * the ADR catalogue must be exactly the state the freeze record expects.
    The expectation is recorded, not guessed from a set of acceptable values,
    so a regression that drops an ADR out of `adrs/` is caught instead of
    being re-signed away.

The manifest's hash range covers the delivered artefacts only:
`handoff/`, `takeover/dsh-b/`, `docs/releases/v4.1.0/refactoring/`, plus the
freeze record and the package README.  The rest of `takeover/`'s top level is
an active workspace (the design document, the implementation plan, the
owner's running decisions) -- DSH-B writes there constantly, and every one of
those writes used to make the manifest drift.

The invariants this gate asserts are raised explicitly instead of being
written as `assert` statements wherever they were rewritten, because
`python -O` strips `assert` and would turn them into no-ops; the tool also
refuses to run under `python -O` at all.
"""
import argparse
import copy
import datetime
import hashlib
import inspect
import json
import linecache
import pathlib
import re
import subprocess
import sys
import traceback

ROOT = pathlib.Path(__file__).resolve().parents[5]
PLAN = ROOT / "docs/releases/v4.1.0/refactoring"
HAND = ROOT / "docs/refactor/v4.1.0-c/handoff"
V41 = ROOT / "docs/refactor/v4.1.0-c"
FREEZE_REL = "docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json"
FREEZE = ROOT / FREEZE_REL
HARA_REL = "docs/releases/v4.1.0/refactoring/audit/HARA-410/evidence/MANIFEST.json"
HARA_ROOT = PLAN / "audit/HARA-410"
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
    V41 / "takeover/dsh-b",
    PLAN,
)

# Files outside MANIFEST_ROOTS that are still covered: the freeze record is
# the sole authority for base_sha/tree_sha, and the README sits next to the
# handoff package.  Everything else under takeover/'s top level stays out.
MANIFEST_EXTRA_FILES = (
    FREEZE,
    V41 / "README.md",
)

# The manifest excludes itself and two receipt files.  They are excluded by
# path under the handoff directory, not by name: a governance record
# elsewhere that happens to share a name must not silently escape coverage.
MANIFEST_EXCLUDED = (
    "docs/refactor/v4.1.0-c/handoff/10_HANDOFF_MANIFEST.json",
    "docs/refactor/v4.1.0-c/handoff/validation.json",
    "docs/refactor/v4.1.0-c/handoff/PUBLICATION_RECEIPT.json",
)

# The working tree may only carry changes under these prefixes -- the same
# roots as the manifest's hash range, so a change the manifest cannot see is
# also a change the gate refuses.  `takeover/`'s top level is allowed (it is
# the active workspace) but nothing outside these prefixes is.
ALLOWED_CHANGE_PREFIXES = (
    "docs/releases/v4.1.0/refactoring/",
    "docs/refactor/v4.1.0-c/handoff/",
    "docs/refactor/v4.1.0-c/takeover/",
)

# The tier vocabulary.  Matching is on the whole token, never on a substring:
# "NOT APPROVED", "NOT_APPROVED" and "UNAPPROVED" are all PROPOSED, and a
# sentence that merely mentions APPROVED is not an approval.  The Chinese
# spellings are the owner's own wording (ADR-DECISION-RECORD.md); they are
# listed here so the next task can write either form and the gate will
# recognise it.
ADR_TIER_BY_TOKEN = {
    "APPROVED": "APPROVED",
    "无条件批准": "APPROVED",
    "APPROVED_DIRECTION": "APPROVED_DIRECTION",
    "仅批准方向": "APPROVED_DIRECTION",
    "批准方向": "APPROVED_DIRECTION",
    "CONDITIONALLY_APPROVED": "CONDITIONALLY_APPROVED",
    "条件批准": "CONDITIONALLY_APPROVED",
}

# The tiers, most specific first.  "APPROVED" is a prefix of
# "APPROVED_DIRECTION", so the tiers are tested most-specific first.
ADR_TIERS = ("APPROVED_DIRECTION", "CONDITIONALLY_APPROVED", "APPROVED")

# The ADR the "tier overreach" mutation arm points at.  The arm supplies its
# own tier map, so it does not depend on what the ADR text currently says.
# The owner's ruling on each ADR lives in
# takeover/dsh-b/ADR-DECISION-RECORD.md; this gate points at that ledger
# rather than keeping a second copy of it.
TIER_OVERREACH_ADR = "ADR-410-003"

# An ADR reference inside an `Approvals:` element.
ADR_REF_RE = re.compile(r"ADR-410-\d{3}")


class HandoffError(Exception):
    """A handoff invariant does not hold."""

    def __init__(self, message, call_site=None):
        super().__init__(message)
        self.call_site = call_site


def require(condition, message):
    """Assert-like helper that survives `python -O` (which strips `assert`).

    The call site is attached to the error: the failing frame of a
    `require()`-raised error is the `raise` inside this helper, which would
    name every failure the same place.
    """
    if not condition:
        frame = inspect.currentframe().f_back
        site = None
        if frame is not None:
            line = linecache.getline(frame.f_code.co_filename, frame.f_lineno).strip()
            site = f"{frame.f_code.co_filename}:{frame.f_lineno}: {line}"
        raise HandoffError(message, site)


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

def frozen_blob_sha(base, rel):
    """The sha256 of a path as it was in the frozen base commit.

    The frozen commit's content cannot be rewritten by `--finalize`, so this
    is an anchor that does not move with the artefact it checks: editing the
    HARA manifest and re-signing the package does not move it.
    """
    raw=subprocess.check_output(["git","show",f"{base}:{rel}"],cwd=ROOT)
    return hashlib.sha256(raw).hexdigest()

def body_lines(text):
    """The lines of an ADR body, with fenced code blocks removed.

    A fenced block that happens to contain a line starting with `Status:`
    must not be mistaken for the ADR's own status line.
    """
    lines=[]
    fenced=False
    for line in text.splitlines():
        if line.lstrip().startswith("```"):
            fenced=not fenced
            continue
        if not fenced:
            lines.append(line)
    return lines

def tier_of(status_value):
    """The tier of a `Status:` value, matched on the whole token.

    The value is cut at the first sentence stop and at any parenthesis, then
    its first delimited token is looked up in the vocabulary.  Anything that
    is not in the vocabulary -- including "NOT APPROVED" and a sentence that
    merely mentions APPROVED -- is PROPOSED.
    """
    head=re.split(r"[。；;（(]",status_value)[0]
    tokens=re.split(r"[\s/、,，]+",head.strip())
    token=tokens[0].strip("：:。．.、*`_") if tokens and tokens[0] else ""
    return ADR_TIER_BY_TOKEN.get(token,"PROPOSED")

def split_approvals(value):
    """The elements of an `Approvals:` field."""
    value=value.strip()
    if value.startswith("["):
        value=value[1:]
    if value.endswith("]"):
        value=value[:-1]
    return [element.strip() for element in re.split(r"[,，;；]",value) if element.strip()]

def parse_adr_status(path, real_ids):
    """(tier, approval refs) parsed out of an ADR's own Status/Approvals line.

    The ADR text is the authority: the catalogue in
    01_ARCHITECTURE_BASELINE.json is a projection of it, never a second source
    of truth.

    Every element of the `Approvals:` field must name a real ADR.  An element
    that names no ADR at all, or names one that does not exist, is rejected --
    never silently dropped, because a dropped element is an approval the
    catalogue no longer accounts for.
    """
    lines=body_lines(path.read_text())
    status_lines=[line for line in lines if line.lstrip().startswith("Status:")]
    require(len(status_lines)==1,
            f"{path.name} must carry exactly one 'Status:' line outside a code fence, "
            f"found {len(status_lines)}")
    status_line=status_lines[0]
    status_value=status_line.split("Status:",1)[1]
    tier=tier_of(status_value)
    approvals=None
    if "Approvals:" in status_line:
        approvals=status_line.split("Approvals:",1)[1]
    else:
        for line in lines:
            if line.lstrip().startswith("Approvals:"):
                approvals=line.split("Approvals:",1)[1]
                break
    refs=[]
    if approvals is not None:
        for element in split_approvals(re.split(r"[。；;]",approvals)[0]):
            found=ADR_REF_RE.findall(element)
            require(found,
                    f"{path.name}: the Approvals field names {element!r}, which is not an ADR "
                    f"reference -- the field may only name ADRs")
            for ref in found:
                require(ref in real_ids,
                        f"{path.name}: the Approvals field names {ref}, which is not a real ADR id")
                refs.append(ref)
    return tier,sorted(set(refs))

def adr_catalogue():
    """{ADR id: {"tier":…, "approval_refs":…}} for every ADR file in `adrs/`."""
    paths=sorted((PLAN/"adrs").glob("*.md"))
    ids={path.stem for path in paths}
    catalogue={}
    for path in paths:
        tier,refs=parse_adr_status(path,ids)
        catalogue[path.stem]={"tier":tier,"approval_refs":refs}
    return catalogue

def adr_tiers():
    """{ADR id: tier} for every ADR file in `adrs/`."""
    return {adr_id:entry["tier"] for adr_id,entry in adr_catalogue().items()}

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
      R3  a direction-only ADR may not be cited by a task that is authorised
          for execution -- the implementation is reviewed separately
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
    try:
        from jsonschema import Draft202012Validator
    except ImportError as exc:
        raise HandoffError(
            f"jsonschema is not installed ({exc}); install the pinned dependency with "
            f"`pip install -r docs/releases/v4.1.0/refactoring/tools/requirements.txt`") from exc
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
        require(t["base_sha"]==base,
                f"task {t['task_id']} is bound to base_sha {t['base_sha']}, but the frozen "
                f"baseline is {base}")
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
    catalogue=adr_catalogue()
    adrs=[]
    for adr_id in sorted(catalogue):
        path=PLAN/"adrs"/f"{adr_id}.md"
        adrs.append({"id":adr_id,"path":str(path.relative_to(ROOT)),"sha256":sha(path),
                     "status":catalogue[adr_id]["tier"],
                     "approval_refs":catalogue[adr_id]["approval_refs"]})
    dump(HAND/"01_ARCHITECTURE_BASELINE.json",{"version":"C-HANDOFF-1","base_sha":base,"tree":record["tree_sha"],"source_run":bindings["source_run"],"timestamp":stamp,"stage":"ALPHA","hara_manifest":{"path":HARA_REL,"sha256":sha(ROOT/HARA_REL)},"source_bindings":{"path":str((PLAN/"evidence/SOURCE_BINDINGS.json").relative_to(ROOT)),"sha256":sha(PLAN/"evidence/SOURCE_BINDINGS.json")},"adrs":adrs,"architecture_principle":"OWNER_ACCEPTED_C","adr_approval":aggregate_adr_approval(adrs),"dispatch_allowed":False,"pending_owner_decisions":["DSH takeover","18-task budget","LOCAL_ONLY enforcement","ADR digests","isolation/product scope","format/sidecar","resource windows","independent reviewer"]})
    dump(HAND/"04_TASK_DEPENDENCY_DAG.json",{"kind":"HASH_BOUND_REFERENCE_NOT_SECOND_DAG","path":str((PLAN/"REFACTOR_TASK_DAG.json").relative_to(ROOT)),"sha256":sha(PLAN/"REFACTOR_TASK_DAG.json"),"dispatch_allowed":False})
    files={}
    for directory in MANIFEST_ROOTS:
        for p in sorted(directory.rglob("*")):
            if p.is_file() and "__pycache__" not in p.parts:
                files[str(p.relative_to(ROOT))]=sha(p)
    for p in MANIFEST_EXTRA_FILES:
        if p.is_file():
            files[str(p.relative_to(ROOT))]=sha(p)
    files={path:digest for path,digest in files.items() if path not in MANIFEST_EXCLUDED}
    dump(HAND/"10_HANDOFF_MANIFEST.json",{"version":"C-HANDOFF-1","source_agent":"Codex GPT-6.1 Sol","source_run":bindings["source_run"],"timestamp":stamp,"base_sha":base,"source_uri":"http://192.168.0.252:3000/openclaw/sqlrustgo","conflict_resolution":"Owner approval and live DSH policy required","status":"CANDIDATE_NOT_AUTHORIZED","files":files,"file_count":len(files),"manifest_self_hash":"External sha256sum; excluded to avoid recursion","exclusions":list(MANIFEST_EXCLUDED)})
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

def expected_catalogue(record):
    """The ADR catalogue state the freeze record expects.

    The expectation is recorded, not derived from a set of acceptable values:
    a band that accepts either value without saying which one is expected is
    not an assertion.
    """
    expectation=record.get("adr_catalogue")
    require(expectation is not None,
            f"{FREEZE_REL} does not record the expected ADR catalogue state "
            f"('adr_catalogue'); the gate cannot tell a regression from the "
            f"expected state")
    require(isinstance(expectation.get("expected_ids"),list) and expectation["expected_ids"],
            f"{FREEZE_REL}: adr_catalogue.expected_ids must be a non-empty list")
    require(isinstance(expectation.get("expected_size"),int),
            f"{FREEZE_REL}: adr_catalogue.expected_size must be an integer")
    return expectation

def check():
    dag,native=load_tasks()
    record=freeze_record()
    base=record["base_sha"]
    expectation=expected_catalogue(record)
    catalogue=adr_catalogue()
    tiers={adr_id:entry["tier"] for adr_id,entry in catalogue.items()}
    validate(dag,native,tiers,base)
    rejected=[]
    reasons={}
    mutations=[]
    # Each arm is validated against the tier map it is named for.  The "tier
    # overreach" arm supplies its own map, so it exercises the rule it is
    # named for no matter what the ADR texts currently say.
    arm_tiers={"tier_overreach":{**tiers,TIER_OVERREACH_ADR:"APPROVED_DIRECTION"}}
    d=copy.deepcopy(dag);d["tasks"][0]["dependencies"]=["C01-18"];mutations.append(("cycle",d,native))
    d=copy.deepcopy(dag);d["tasks"][0]["dependencies"]=["MISSING"];mutations.append(("missing",d,native))
    d=copy.deepcopy(dag);d["tasks"][0]["dispatch_allowed"]=True;mutations.append(("unapproved_dispatch",d,native))
    d=copy.deepcopy(dag);d["tasks"][0]["approved_adr_refs"]=["fake"];mutations.append(("unapproved_adr",d,native))
    d=copy.deepcopy(dag);d["tasks"][12]["human_gate"]["required"]=False;mutations.append(("human_gate_removed",d,native))
    d=copy.deepcopy(dag);d["tasks"][0]["model_policy"]="LOCAL_FIRST";mutations.append(("cloud_policy",d,native))
    d=copy.deepcopy(dag);d["tasks"][0]["milestone_id"]="C-ALPHA-02";mutations.append(("later_stage_task",d,native))
    n=copy.deepcopy(native);n[0]["model_policy"]="LOCAL_ONLY";mutations.append(("native_extra_field",dag,n))
    d=copy.deepcopy(dag);d["tasks"][0]["approved_adr_refs"]=[TIER_OVERREACH_ADR];d["tasks"][0]["authorized_for_execution"]=True;mutations.append(("tier_overreach",d,native))
    d=copy.deepcopy(dag);d["tasks"][0]["approved_adr_refs"]=[TIER_OVERREACH_ADR];mutations.append(("unapproved_adr_still_proposed",d,native))
    for name,d,n in mutations:
        try:validate(d,n,arm_tiers.get(name,tiers),base)
        except Exception as exc:
            rejected.append(name)
            reasons[name]=rejection_reason(exc)
        else:raise AssertionError("mutation accepted: "+name)
    baseline=read(HAND/"01_ARCHITECTURE_BASELINE.json")
    require(sha(ROOT/HARA_REL)==frozen_blob_sha(base,HARA_REL),
            f"{HARA_REL} no longer matches the frozen base commit's copy of it "
            f"({base[:10]}); the HARA evidence input was rewritten under the handoff")
    m=read(ROOT/HARA_REL)
    for p,h in m["files"].items():assert sha(HARA_ROOT/p)==h,p
    manifest=read(HAND/"10_HANDOFF_MANIFEST.json")
    for p,h in manifest["files"].items():assert sha(ROOT/p)==h,p
    ref=read(HAND/"04_TASK_DEPENDENCY_DAG.json");assert sha(ROOT/ref["path"])==ref["sha256"]
    # The catalogue must be exactly the state the freeze record expects: a
    # regression that drops an ADR out of `adrs/` is caught here, not
    # re-signed away.
    expected_ids=set(expectation["expected_ids"])
    actual_ids=set(catalogue)
    require(actual_ids==expected_ids,
            f"the ADRs in adrs/ are {sorted(actual_ids)}, but {FREEZE_REL} expects "
            f"{sorted(expected_ids)}; update the freeze record's 'adr_catalogue' "
            f"expectation if this change is intended")
    require(len(baseline["adrs"])==expectation["expected_size"],
            f"the ADR catalogue holds {len(baseline['adrs'])} entry/entries, but "
            f"{FREEZE_REL} expects {expectation['expected_size']}")
    require(("ADR-410-011" in actual_ids)==bool(expectation.get("adr_410_011_in_adrs")),
            f"ADR-410-011 is {'in' if 'ADR-410-011' in actual_ids else 'not in'} adrs/, but "
            f"{FREEZE_REL} records adr_410_011_in_adrs="
            f"{expectation.get('adr_410_011_in_adrs')}")
    require({entry["id"] for entry in baseline["adrs"]}==expected_ids,
            f"the ADR catalogue lists {sorted(entry['id'] for entry in baseline['adrs'])}, "
            f"but {FREEZE_REL} expects {sorted(expected_ids)}")
    for entry in baseline["adrs"]:
        require(entry["id"] in catalogue,
                f"the catalogue lists {entry['id']}, which has no file in adrs/")
        require(sha(ROOT/entry["path"])==entry["sha256"],
                f"the catalogue's digest for {entry['id']} does not match {entry['path']}")
        require(entry["status"]==catalogue[entry["id"]]["tier"],
                f"the catalogue records {entry['id']} as {entry['status']}, but the ADR's own "
                f"text reads {catalogue[entry['id']]['tier']}")
        require(entry["approval_refs"]==catalogue[entry["id"]]["approval_refs"],
                f"the catalogue records {entry['id']}'s approvals as "
                f"{entry['approval_refs']}, but the ADR's own Approvals field reads "
                f"{catalogue[entry['id']]['approval_refs']}")
        require(not entry["approval_refs"] or set(entry["approval_refs"])<=set(catalogue),
                f"{entry['id']} cites approval references that are not real ADR ids: "
                f"{sorted(set(entry['approval_refs'])-set(catalogue))}")
    cases=read(PLAN/"TEST_COVERAGE_MATRIX.json")["cases"]
    assert len(cases)==75
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
    # The catalogue aggregate says how far the owner's ADR decision got; the
    # state says whether the handoff may treat it as settled.  A task that
    # still has no ADR mapped is an unsettled handoff, whatever the ADR texts
    # say.
    state="PENDING" if pending or baseline["adr_approval"]=="PENDING" else baseline["adr_approval"]
    notes=[]
    if pending:
        notes.append(f"{len(pending)} of {len(dag['tasks'])} task(s) still carry an empty "
                     f"approved_adr_refs -- the owner's ADR decision is PENDING, not verified")
    if not expectation.get("adr_410_011_in_adrs"):
        notes.append("ADR-410-011 is not merged into adrs/ yet (O7); the freeze record "
                     "expects it to be absent and Task 4 owns the relocation")
    return {"kind":"OFFLINE_DELIVERY_VALIDATION_NOT_EXECUTION_CERTIFICATION","source_run":"C-ALPHA-01-HANDOFF-20261120","timestamp":datetime.datetime.now(datetime.timezone.utc).isoformat(),"native_tasks":len(native),"adrs":len(baseline["adrs"]),"adr_catalogue_matches_adrs_dir":actual_ids==expected_ids,"adr_catalogue_expected_ids":sorted(expected_ids),"adr_011_merged_into_adrs":bool(expectation.get("adr_410_011_in_adrs")),"adr_tiers":tier_counts,"adr_approval":baseline["adr_approval"],"adr_approval_state":state,"tasks_with_empty_approved_adr_refs":len(pending),"tasks_pending_adr_approval":pending,"pilot_input_cases":len(cases),"hara_hashed_files":len(m["files"]),"manifest_bound_files":len(manifest["files"]),"task_dag":"ACYCLIC_NO_MISSING_DEPENDENCIES","mutation_rejections":rejected,"mutation_rejection_reasons":reasons,"local_links":links,"authorized_dispatch_count":sum(1 for x in dag["tasks"]+dag["future_epics"] if x["dispatch_allowed"]),"authorized_tokens":dag["budget"]["authorized_tokens"],"proposed_token_upper":sum(x["token_budget"]["upper_all_attempts"] for x in dag["tasks"]),"handoff_manifest_sha256":sha(HAND/"10_HANDOFF_MANIFEST.json"),"notes":notes}

def main():
    parser=argparse.ArgumentParser(description="Offline validation of the v4.1.0 C-ALPHA-01 handoff package.")
    parser.add_argument("--finalize",action="store_true",help="re-sign the handoff manifests")
    parser.add_argument("--check",action="store_true",help="validate the handoff package")
    args=parser.parse_args()
    if sys.flags.optimize:
        print("refusing to run under python -O: assert statements are stripped, this gate would be a no-op")
        return 2
    if not (args.check or args.finalize):
        parser.error("nothing to do: pass --check and/or --finalize")
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
        site=getattr(exc,"call_site",None)
        if site is None:
            frames=traceback.extract_tb(exc.__traceback__)
            if frames:
                frame=frames[-1]
                site=f"{frame.filename}:{frame.lineno}: {(frame.line or '').strip()}"
        if site:
            head,_,rest=site.partition(":")
            try:
                head=str(pathlib.Path(head).relative_to(ROOT))
            except ValueError:
                pass
            print(f"      at {head}:{rest}")
        return 1
    return 0

if __name__=="__main__":
    sys.exit(main())
