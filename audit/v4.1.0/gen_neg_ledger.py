#!/usr/bin/env python3
"""Generator for docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl (P1-3 ledger).

Kept in the repo so the ledger is reproducible from a clean checkout:

    python3 audit/v4.1.0/gen_neg_ledger.py <evidence-run-id>

The generator asserts that every scope gate (stage_required union the
SOAK/PERF/COMPAT/TPCH/STABILITY/CRASH/UPGRADE/CHAOS/QPS/SYSBENCH names in
GATE_CATALOG) has exactly one row, so the ledger cannot silently shrink.
Not a gate: it never runs in ci_gate.sh and never changes the denominator.

Three modes (2026-11-25, M-6).  The tool used to have exactly ONE mode, and it
was destructive: it overwrote the committed ledger in place, took the evidence
run id straight from argv[1] without checking it, and printed a summary that
looked equally healthy either way.  A mistyped argument could therefore
rewrite a committed ledger without the tool saying so.

    (default)      regenerate the committed ledger IN PLACE.  The behaviour is
                   unchanged on purpose -- it is how the ledger has always been
                   produced -- but the summary now states EXPLICITLY which path
                   was written and which evidence run id the rows cite, so a
                   mistake cannot pass unnoticed.
    --check        NON-DESTRUCTIVE.  Regenerate in memory and compare the
                   result with the committed ledger BYTE FOR BYTE.  Exit 0 when
                   identical; exit 1 with a diff summary when not.  It NEVER
                   writes a file.
    --out <path>   write the regenerated ledger to <path> instead of
                   overwriting the committed one (for manual comparison).

The evidence run ids are validated BEFORE anything is written: a run id must
look like <YYYYMMDD>-<lowercase-slug> AND must exist under
audit/v4.1.0/runs/.  A mistyped argument (e.g. `--help`) is therefore REJECTED
with a human-readable message and a non-zero exit code, and NO file is
written.
"""
import hashlib
import json
import pathlib
import re
import sys

# The repository root, derived from this file's own location: the generator
# must be able to write into WHICHEVER checkout it is run from, otherwise
# "the ledger is reproducible from a clean checkout" is false in a second
# sense (it used to point at one machine's worktree).
REPO = pathlib.Path(__file__).resolve().parents[2]
CAT = REPO / "docs/releases/v4.1.0/GATE_CATALOG.jsonl"
OUT = REPO / "docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl"
RUNS = REPO / "audit/v4.1.0/runs"
# Two evidence runs, because two different batches of fixtures were executed in
# two different runs:
#   RUN  -- the historical batch (fx01..fx08, fx14, the self-test ones)
#   RUN8 -- the v4.1.0 readiness plan's Task 8 batch (fx02 re-run + fx15..fx20
#           and their two characterization companions fx21/fx22)
# Both are arguments so that a re-run against another tree can name its own
# runs; the defaults are the ones the committed ledger cites.
DEFAULT_RUN = "20261105-negfix-003"
DEFAULT_RUN8 = "20261124-task8-alpha-negfix-001"
ISSUE = "issue#5598"
NEG = "audit/v4.1.0/neg/"
NAME = re.compile(r"SOAK|PERF|COMPAT|TPCH|STABILITY|CRASH|UPGRADE|CHAOS|QPS|SYSBENCH", re.I)
# The shape every evidence run id in audit/v4.1.0/runs/ actually has
# (140/140 measured).  A run id that does not match it is a typo, not a run.
RUN_ID_RE = re.compile(r"^[0-9]{8}-[a-z0-9]+(-[a-z0-9]+)*$")

USAGE = """用法：
  python3 audit/v4.1.0/gen_neg_ledger.py                 就地重出已提交的台账（默认行为）
  python3 audit/v4.1.0/gen_neg_ledger.py --check         只校验：逐字节比对，绝不写文件
  python3 audit/v4.1.0/gen_neg_ledger.py --out <path>    生成到 <path>，不覆写已提交的台账
  python3 audit/v4.1.0/gen_neg_ledger.py <run-id> [<run8-id>]

位置参数是**证据 run id**：形态须为 <YYYYMMDD>-<小写 slug>，且必须存在于
audit/v4.1.0/runs/ 下。默认值：%s / %s""" % (DEFAULT_RUN, DEFAULT_RUN8)

cat = {}
for line in CAT.read_text(encoding="utf-8").splitlines():
    if line.strip():
        r = json.loads(line)
        cat[r["gate"]] = r
scope = sorted({g for g, r in cat.items()
                if r.get("class") == "stage_required" or NAME.search(g)})

MUT = {
 "scripts/gate/check_gate_executability.sh": ("fx01-executability-missing-script.sh", "static",
   "STAGE_CONFIG 的 required gate 脚本被删掉时 C1 存在性判据变红（实测门禁 rc=1）。该门禁另有 A/B/C 三臂自带 selftest（scripts/gate/gate_executability.py）"),
 "scripts/gate/check_drift_not_pass.sh": ("fx03-drift-not-pass-new-antipattern.sh", "static",
   "新增一个用 `cargo test ... > log 2>&1 || true` 吞退出码的门禁脚本 ⇒ 相对 tests/baseline/drift_baseline.json 出现新反例，门禁变红（实测 rc=1）"),
 "scripts/gate/check_tests_dont_write_archived_evidence.sh": ("fx04-tests-write-archived-evidence.sh", "static",
   "在 tests/ 放一个直接 fs::write 到 docs/releases/v4.1.0/evidence/ 的测试 ⇒ L1 静态判据变红（实测 rc=1）；自带 L1 selftest 是第二条证据"),
 "scripts/gate/check_cross_version_debt.sh": ("fx05-cross-version-debt-doc-missing.sh", "static",
   "删掉 CROSS-VERSION-DEBT.md / INT5 清单的全部候选路径 ⇒ 按缺 SSOT 变红（实测 rc=1）"),
 "scripts/gate/check_int_debt.sh": ("fx06-int-debt-doc-missing.sh", "static",
   "删掉 CROSS-VERSION-DEBT.md 全部候选路径 ⇒ 按缺文档 exit 1（实测 rc=1）"),
 "scripts/gate/check_docs_links.sh": ("fx07-docs-links-broken.sh", "static",
   "在入口文档 README.md 加一条指向不存在路径的 markdown 链接 ⇒ 变红（实测 rc=1）"),
}
# The Task 8 batch (v4.1.0 readiness plan): six ALPHA-stage rows whose evidence
# is the Task 8 evidence run.  fx02 is in here because the Task 8 round
# re-ran it (it was extended with a two-layer proof and a discriminating
# assertion), so its evidence_run/evidence_log must point at the run that
# actually executed it.
#
# Every row here states the property the fixture ACTUALLY established.  Where
# the fixture also proved the opposite (the gate cannot see some failure mode),
# that is recorded as a **known hole** -- a field the ledger validator checks
# (the referenced fixture must declare `# expect: green-characterization`) --
# and never as a `catches` claim.
MUT8 = {
 "scripts/gate/check_anti_ignore_gate.sh": ("fx02-anti-ignore-registry-drift.sh", "static",
   "往 tests/*.rs 加一个真实 #[ignore] 而不登记进 tests/baseline/ignore_registry.json ⇒ registry drift 判据变红；夹具另断言门禁报出的 drift 数字随注入的 #[ignore] 变化（该门禁在冻结基线上本来就是红的：树里 136 个真实 #[ignore] vs 登记表登记 135）", None, None),
 "scripts/gate/check_alpha_v410.sh": ("fx15-alpha-v410-composite-entry-doc-missing.sh", "composite",
   "删掉 docs/releases/v4.1.0/STAGE.yaml 后 composite 的 ENTRY_STATUS 变成非 0、并打印 'STATUS: ALPHA GATE BLOCKED'（改前是 ALPHA ENTRY PASS / ALPHA QUALITY BLOCKED）⇒ entry-doc-missing 可证伪；两层 RED 同源于一次 run_gates.py execute 的 results.jsonl",
   "删 docs/releases/v4.1.0/STAGE.yaml 后 composite 必须变红（其 entry 分量已由 fx08 证伪）；composite 实测 200s 未跑完，需要热构建泳道", None),
 "scripts/gate/check_arch3_no_bypass.sh": ("fx17-arch3-no-bypass-dml-guard-dropped.sh", "cargo",
   "ExecutionEngine 的 DML 调用不再声明自己的路径（assert_path_for_dml marker 被摘掉）后门禁的 marker 计数掉到 2 < 3 ⇒ dml-guard-dropped 可证伪。反方向（新增一处绕过 DML 调用）门禁看不见，单独由 fx22 证明并记成 known_hole",
   "ExecutionEngine 的 DML 调用不再声明自己的路径（assert_path_for_dml marker 被摘掉）⇒ 必须变红",
   ("audit/v4.1.0/neg/fx22-arch3-added-bypass-invisible-characterization.sh",
    "门禁只数 marker（src/execution_engine*.rs 里 assert_path_for_dml 计数 >= 3、三个 EE DML 函数各自带 marker、openclaw 端点 marker >= 2、marker 函数是 pub），**新增**一处绕过 DML 调用不改变其中任何一项 ⇒ 实测新增后门禁仍然绿（fx22）。改判据属属主决策（#5595 / #5598）")),
 "scripts/gate/check_arch_invariants.sh": ("fx16-arch-invariants-invariant-impl-removed.sh", "cargo",
   "摘掉 C-ARCH-05 的实现对象 src/execution_engine.rs 后，门禁必须 fail closed（'cannot be evaluated - target file not found'）⇒ invariant-impl-removed 可证伪",
   "摘掉一条 invariant 的实现 ⇒ 必须变红", None),
 "scripts/gate/check_g1_tpch_baseline.sh": ("fx20-g1-tpch-baseline-hash-tampered.sh", "runtime",
   "篡改 tests/tpch_hashes_v380.json 的 SF1 基线数字后，门禁比对的就是被篡改的那个数 ⇒ hash-tampered 可证伪。对照是绿的：冻结基线上门禁 exit 0、算出的哈希等于基线数字 7439ce80…，所以红可归因于篡改",
   "篡改 SF1 基线数字 ⇒ 必须变红；需要 SF1 数据集与热 target", None),
 "scripts/gate/check_sql_corpus_gate.sh": ("fx19-sql-corpus-gate-corpus-truncated.sh", "cargo",
   "截断语料库（sql_corpus/**/*.sql 全部截成 0 字节）后语料量到 0.0% 通过率、门禁判 'FAIL: pass rate 0.0% < 80.0% threshold' ⇒ corpus-truncated 可证伪。篡改 expected 输出对该门禁不可见，单独由 fx21 证明并记成 known_hole",
   "截断语料库 ⇒ 必须变红",
   ("audit/v4.1.0/neg/fx21-sql-corpus-expected-tamper-invisible-characterization.sh",
    "语料 harness 解析 `-- EXPECT: rows <n>` 时取 expect.split_whitespace().next()（第一个词是 'rows'），parse 必然失败 ⇒ expected_rows 一直是 None ⇒ success 回落到 unwrap_or(true) ⇒ **期望行数从来没有被比较过**。实测篡改 1476 条 EXPECT 后门禁仍报 98.2% 仍然绿（fx21）。改 harness 就是改门禁在量什么，属属主决策（#5595 / #5598）")),
}
SELF = {
 "scripts/gate/check_commit_message.sh": ("scripts/gate/check_commit_message.sh",
   "自带 --selftest：临时 git 仓里造一条坏提交信息，checker 必须拒绝（实跑 rc=0，见 004 日志）"),
 "scripts/gate/check_coverage_single_implementation.sh": ("scripts/gate/check_coverage_single_implementation.sh",
   "自带 --selftest：用与线上检查同一份 scan_hits 代码跑临时样本树，三臂（直调工具未检出 / 清单夹具未检出 / 只调 helper 的门禁被误报）"),
 "scripts/gate/check_gate_test_integrity.sh": ("scripts/gate/check_gate_test_integrity.sh",
   "自带 --selftest：`cargo test ... || true` 掩码检测器用自己的夹具跑真实规则（#5553），实跑 rc=0"),
}
UNF = {
 "scripts/gate/check_architecture_freeze.sh": ("fx09-architecture-freeze-characterization.sh",
   "脚本末尾是 `exit 0  # 不阻塞，因为是改进项`：夹具把 crates/ 里的 DriftGate 引用全部改名后，门禁自己打印 `FAIL (0 references - path not established)`，退出码仍是 0 ⇒ 这道 GA required gate 结构上不可能变红。改判定属 AGENTS 3.2，且会让 GA 清单立刻红，属 owner 决策（#5595 / #5598）"),
}
PEND = {
 "scripts/gate/check_anti_fabrication.sh": ("cargo", "删掉一条降级声明的 run 引用，或把白名单里一条已知失败测试改成真实失败 ⇒ 必须变红；先要 cargo check 热 target"),
 "scripts/gate/check_arch_sem_debt.sh": ("cargo", "把语义债表里一条 CLOSED 改回 ACTIVE 且无计划 ⇒ 必须变红"),
 "scripts/gate/check_backup_restore.sh": ("runtime", "让 backup→restore 往返丢行（截断备份）⇒ 必须变红；需要构建产物"),
 "scripts/gate/check_beta_gate.sh": ("composite", "把 BETA 覆盖率实测降到 COVERAGE_MIN_BETA 之下 ⇒ 必须变红"),
 "scripts/gate/check_coverage_v312.sh": ("cargo", "把某个 crate 的覆盖率表替换成低于阈值的值 ⇒ 必须变红；llvm-cov 实测 >720s 未跑完"),
 "scripts/gate/check_full_gate_verification.sh": ("cargo", "让它调用的某道子门禁必然变红（复用 fx04 的夹具文件），composite 必须报该子门禁 FAIL 而不是吞掉退出码"),
 "scripts/gate/check_g16_compatibility.sh": ("external", "把一个兼容性用例期望改错或断掉 oracle ⇒ 必须变红；需要外部 MySQL/Oracle"),
 "scripts/gate/check_gate_self_verification.sh": ("cargo", "注入一个 `0 tests run` 的 cargo test 目标 ⇒ 必须按它自己声明的 V1 判红"),
 "scripts/gate/check_int2_no_orphan.sh": ("cargo", "把孤岛 executor 的引用改名清零 ⇒ 必须变红"),
 "scripts/gate/check_int3_single_expr.sh": ("cargo", "恢复 expr crate 的第二实现 ⇒ 必须变红"),
 "scripts/gate/check_integration_gate.sh": ("cargo", "把一条集成测试标 #[ignore] 且不登记 ⇒ 必须变红"),
 "scripts/gate/check_p12_crash_test.sh": ("runtime", "关掉一处 WAL flush 后做崩溃注入 ⇒ 必须变红；需要构建产物"),
 "scripts/gate/check_p14_upgrade_test.sh": ("runtime", "用旧版本数据文件跑升级路径并破坏迁移步骤 ⇒ 必须变红"),
 "scripts/gate/check_p21_audit_log.sh": ("runtime", "篡改一条审计日志的 hash 链 ⇒ 必须变红"),
 "scripts/gate/check_p22_time_travel.sh": ("runtime", "让时间旅行读回错误快照 ⇒ 必须变红"),
 "scripts/gate/check_p23_hash_chain.sh": ("runtime", "截断 hash chain 中段 ⇒ 必须变红"),
 "scripts/gate/check_rc_ga_gate.sh": ("composite", "删掉一条 GA required gate 的执行点 ⇒ 必须变红。注意 GATE_CATALOG 里它的 neg_fixture=True 是关键字误命中（命中的是 `ADR-004-negative-evidence.md` 这个文件名），不是夹具"),
 "scripts/gate/check_sem1_savepoint.sh": ("runtime", "让 SAVEPOINT 回滚漏放锁 ⇒ 必须变红"),
}
EXTRA = {
 "scripts/gate/check_4019_sysbench_baseline.sh": ("external", "把 sysbench 跑成 0 TPS 的空会话 ⇒ 必须变红"),
 "scripts/gate/check_chaos_soak.sh": ("external", "关掉 chaos 注入开关跑同样时长 ⇒ 必须变红；注入开关本身要可核验"),
 "scripts/gate/check_g11_qps.sh": ("runtime", "人为串行化提交把 QPS 压到阈值下 ⇒ 必须变红"),
 "scripts/gate/check_g12_sysbench.sh": ("runtime", "把 sysbench 线程数改成 1 后仍报达标 ⇒ 必须变红"),
 "scripts/gate/check_g13_stability.sh": ("runtime", "在稳定性窗口里注入一次已知崩溃 ⇒ 必须变红"),
 "scripts/gate/check_g14_real_crash.sh": ("runtime", "kill -9 后跳过恢复步骤 ⇒ 必须变红"),
 "scripts/gate/check_g15_perf_report.sh": ("cargo", "删掉 perf report 的关键指标行 ⇒ 必须变红"),
 "scripts/gate/check_g1_tpch_22_22.sh": ("runtime", "篡改 Q22 结果集 ⇒ 必须变红"),
 "scripts/gate/check_ga2_soak_artifact.sh": ("external", "soak 产物时间戳不连续或时长不足 ⇒ 必须变红"),
 "scripts/gate/check_ga_wire_recovery_upgrade.sh": ("runtime", "升级后 wire 恢复路径丢事务 ⇒ 必须变红"),
 "scripts/gate/check_p13_soak_test.sh": ("external", "把 soak 时长缩短到阈值下 ⇒ 必须变红（与 ING-1 的 168h 泳道同一条）"),
 "scripts/gate/check_perf_baseline.sh": ("runtime", "喂一份假的基线 JSON（全部快 10 倍）⇒ 必须变红"),
 "scripts/gate/check_sql_compat.sh": ("external", "把一个兼容用例的期望值改错 ⇒ 必须变红"),
 "scripts/gate/check_sysbench_oltp.sh": ("runtime", "把 OLTP 读写比改成只读后仍按混合负载判定 ⇒ 必须变红"),
 "scripts/gate/check_tpch_known_differences.sh": ("runtime", "删掉一条已登记差异（实际结果仍不同）⇒ 必须变红"),
 "scripts/gate/check_tpch_sf1.sh": ("runtime", "篡改 SF1 任一 query 的期望输出 ⇒ 必须变红"),
 "scripts/gate/check_tpch_sf10.sh": ("runtime", "篡改 SF10 任一 query 的期望输出 ⇒ 必须变红"),
 "scripts/gate/check_upgrade_guide.sh": ("static", "删掉升级指南里一条必需迁移步骤 ⇒ 必须变红"),
 "scripts/gate/check_upgrade_v310_v311.sh": ("runtime", "跳过 v3.10→v3.11 的一步元数据迁移 ⇒ 必须变红"),
 "scripts/gate/check_v312_14_crash_recovery.sh": ("runtime", "崩溃后跳过 WAL 重放 ⇒ 必须变红"),
 "scripts/gate/check_v312_21_mysql_compat.sh": ("external", "把 MySQL 协议握手的一个字段比对摘掉 ⇒ 必须变红"),
}
NOTEXEC = {
 "dead": "该门禁在 GATE_CATALOG 里 class=dead：清单里存在但没有任何执行点。先要有执行点（CI 执行面见 #5597），夹具才有意义",
 "orphan_referenced": "该门禁 class=orphan_referenced：被引用但不在阶段清单里执行。先解决执行点归属（#5597），夹具才有意义",
}


def build_rows(run, run8):
    """Build the ledger rows for the two evidence runs.

    The row set is a pure function of (run, run8) and the gate catalogue: no
    file is touched here.  The "the ledger cannot silently shrink" contract is
    asserted at the end, exactly as before.
    """
    log_fx = "audit/v4.1.0/runs/%s/logs/003-neg-fixtures.stdout" % run
    log_fx8 = "audit/v4.1.0/runs/%s/logs/003-neg-fixtures.stdout" % run8
    log_st = "audit/v4.1.0/runs/%s/logs/004-neg-selftests.stdout" % run

    rows = []
    for gate, (fx, cost, note) in sorted(MUT.items()):
        rows.append({"gate": gate, "stages": cat[gate].get("stages") or [], "class": cat[gate]["class"],
                     "kind": "mutation-verified", "fixture_id": fx[:-3], "fixture": NEG + fx,
                     "evidence_run": run, "evidence_log": log_fx, "cost": cost, "note": note})

    for gate, (fx, cost, note, target, hole) in sorted(MUT8.items()):
        row = {"gate": gate, "stages": cat[gate].get("stages") or [], "class": cat[gate]["class"],
               "kind": "mutation-verified", "fixture_id": fx[:-3], "fixture": NEG + fx,
               "evidence_run": run8, "evidence_log": log_fx8, "cost": cost, "note": note}
        if target:
            row["target"] = target
            row["close_run"] = ISSUE
        if hole:
            row["known_hole"] = hole[0]
            row["known_hole_reason"] = hole[1]
        rows.append(row)

    for r in rows:
        if r["gate"] == "scripts/gate/check_int_debt.sh":
            r["known_hole"] = None
            r["known_hole_closed"] = NEG + "fx10-int-debt-row-unclassifiable-characterization.sh"
            r["known_hole_reason"] = ("【已闭合 2026-11-16】原 fail-open：债务行被改名或删除时只进 FAILED_ITEMS，"
                                      "判定逻辑只看 ACTIVE_COUNT/DEFERRED_WITHOUT_PLAN ⇒ 退出码仍是 0（PASS）；"
                                      "另有一处：状态列非空但未知时回退到「整行扫关键字」，于是 WIP-DONOTUSE 这类行"
                                      "会因 Notes 里的 CLOSED 字样被误判成 CLOSED。修复（#5598 / D-03 裁决）："
                                      "列 4 非空且未知 ⇒ FAIL；列 4 为空才允许整行回退；FAILED_ITEMS 非空 ⇒ FAIL；"
                                      "台账读不到 ⇒ INCOMPLETE(2)。fx10 的期望随之由 green-characterization 翻转为 red，"
                                      "现在它是一条**真正的负向夹具**：不可分类行必须让门禁 rc≠0。"
                                      "治理链：owner 2026-11-16 会话授权 → 双血统征询（qwen flash-next-252 + "
                                      "gpt chatgpt-web，均 ok=true）→ ADR-014 D-06 裁决 D-03（三态 0/1/2）"
                                      "→ 本次落地。")

    rows.append({"gate": "scripts/gate/check_alpha_entry_v4.10.sh",
                 "stages": [], "class": cat["scripts/gate/check_alpha_entry_v4.10.sh"]["class"],
                 "kind": "mutation-verified", "fixture_id": "fx08-alpha-entry-doc-missing",
                 "fixture": NEG + "fx08-alpha-entry-doc-missing.sh",
                 "evidence_run": run, "evidence_log": log_fx, "cost": "static",
                 "note": "check_alpha_v410.sh 的 entry 分量：删 docs/releases/v4.1.0/STAGE.yaml ⇒ E1 判红、门禁 rc=1。该行是 scope 之外（class=ci_reachable）的加测，不等于 composite 本身已证伪"})

    rows.append({"gate": "scripts/gate/check_stage.sh",
                 "stages": [], "class": cat["scripts/gate/check_stage.sh"]["class"],
                 "kind": "mutation-verified", "fixture_id": "fx14-stage-version-uninferable",
                 "fixture": NEG + "fx14-stage-version-uninferable.sh",
                 "evidence_run": run, "evidence_log": log_fx, "cost": "static",
                 "note": "阶段驱动的版本解析：(A) 无可推输入必须 rc=2 且不得先评阶段；(B) 分支不含版本时必须解析 "
                         "v4.1.0（改前实测评到已归档 v3.9.0 并写日志进该目录）；(C) --version 指向不存在的发布线时"
                         "必须 rc=2 且不得创建 docs/releases/v9.9.9。反向变异：同一夹具打修复前 9f82eeb777 三臂全部 FIXTURE-FAIL"})

    selfsab = {
        "scripts/gate/check_commit_message.sh": "fx11-commit-message-detector-crippled.sh",
        "scripts/gate/check_coverage_single_implementation.sh": "fx12-coverage-single-impl-detector-crippled.sh",
        "scripts/gate/check_gate_test_integrity.sh": "fx13-gate-test-integrity-detector-crippled.sh",
    }

    for gate, (impl, note) in sorted(SELF.items()):
        rows.append({"gate": gate, "stages": cat[gate].get("stages") or [], "class": cat[gate]["class"],
                     "kind": "selftest-verified", "selftest_impl": impl,
                     "selftest_neg_fixture": NEG + selfsab[gate],
                     "evidence_run": run, "evidence_log": log_st, "cost": "static", "note": note})

    for gate, (attempt, reason) in sorted(UNF.items()):
        rows.append({"gate": gate, "stages": cat[gate].get("stages") or [], "class": cat[gate]["class"],
                     "kind": "unfalsifiable", "reason": reason,
                     "falsification_attempt": NEG + attempt,
                     "evidence_run": run, "evidence_log": log_fx, "cost": "static"})

    # Pending gates that already HAVE a fixture script trying to close them.  The
    # row still says kind=pending-fixture (the property is not established), but
    # the script is registered so a reader can find it -- and so the lane knows
    # which script to run and what to expect of it.
    pend_fixture = {
        "scripts/gate/check_coverage_v312.sh": ("fx18-coverage-v312-crate-coverage-below-threshold.sh",
           "fx18-coverage-v312-crate-coverage-below-threshold"),
    }

    for gate, (cost, target) in sorted(PEND.items()):
        row = {"gate": gate, "stages": cat[gate].get("stages") or [], "class": cat[gate]["class"],
               "kind": "pending-fixture", "target": target, "close_run": ISSUE, "cost": cost}
        if gate in pend_fixture:
            fx, fid = pend_fixture[gate]
            row["fixture"] = NEG + fx
            row["fixture_id"] = fid
        rows.append(row)

    for gate, (cost, target) in sorted(EXTRA.items()):
        rows.append({"gate": gate, "stages": [], "class": cat[gate]["class"],
                     "kind": "pending-fixture", "target": target, "close_run": ISSUE,
                     "cost": cost, "note": NOTEXEC.get(cat[gate]["class"], "")})

    covered = {r["gate"] for r in rows}
    missing = [g for g in scope if g not in covered]
    assert not missing, "missing scope rows: %s" % missing
    # scope rows + the two scope-extra rows (check_alpha_entry_v4.10.sh entry component,
    # check_stage.sh the stage driver). Extra rows are deliberate over-coverage.
    assert len(rows) == len(scope) + 2, (len(rows), len(scope))
    return rows


def render(rows):
    """The exact bytes the ledger is written with -- one mode, no trailing slack."""
    return "".join(json.dumps(r, ensure_ascii=False, sort_keys=True) + "\n" for r in rows)


def sha256(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def validate_run_id(run_id, label):
    """Reject a run id that is not a real evidence run.  Returns an error string, or None.

    Two checks, both cheap and both fail-closed:
      1. shape   -- <YYYYMMDD>-<lowercase slug>, the shape every one of the 140
                    run directories under audit/v4.1.0/runs/ actually has;
      2. existence -- the run directory must exist, so the rows cannot cite a
                    run that never happened.
    """
    if not RUN_ID_RE.match(run_id):
        return ("%s %r 不是合法的证据 run id：形态须为 <YYYYMMDD>-<小写字母/数字/连字符>"
                "（例如 %s）" % (label, run_id, DEFAULT_RUN))
    if not (RUNS / run_id).is_dir():
        return ("%s %r 形态合法，但 audit/v4.1.0/runs/ 下没有这个 run 目录"
                "（它必须是一次真实发生过的证据 run）" % (label, run_id))
    return None


def parse_args(argv):
    """Hand-rolled on purpose: argparse would swallow `--help` as a flag.

    The whole point of the run-id validation is that a mistyped argument is
    REJECTED instead of being taken for an evidence run.  argparse's own
    `--help` handling would exit 0 and print usage -- fine for a normal tool,
    but it would quietly turn "the operator wanted help" into "the operator
    passed an option" instead of "the operator passed a bogus run id".  So the
    parsing stays explicit and every token that is not a known flag ends up in
    the run-id slot, where the shape check rejects it.
    """
    check = False
    out = None
    positional = []
    i = 0
    while i < len(argv):
        arg = argv[i]
        if arg == "--check":
            check = True
        elif arg == "--out":
            if i + 1 >= len(argv):
                raise ValueError("--out 需要一个路径参数")
            out = argv[i + 1]
            i += 1
        elif arg.startswith("--out="):
            out = arg[len("--out="):]
        elif arg == "--":
            positional.extend(argv[i + 1:])
            break
        else:
            positional.append(arg)
        i += 1
    if len(positional) > 2:
        raise ValueError("多余的位置参数：%s（位置参数最多两个：<run-id> [<run8-id>]）"
                         % " ".join(repr(p) for p in positional[2:]))
    if check and out is not None:
        raise ValueError("--check 与 --out 互斥：--check 绝不写文件")
    run = positional[0] if len(positional) > 0 else DEFAULT_RUN
    run8 = positional[1] if len(positional) > 1 else DEFAULT_RUN8
    return check, out, run, run8


def kinds_of(rows):
    kinds = {}
    for r in rows:
        kinds[r["kind"]] = kinds.get(r["kind"], 0) + 1
    return kinds


def do_check(rows, text):
    """Compare the regenerated ledger with the committed one, byte for byte."""
    if not OUT.exists():
        print("MISMATCH  已提交的台账不存在：%s" % OUT)
        print("          重新生成的版本有 %d 行；用 --out <path> 可以把它落到别处。" % len(rows))
        return 1
    committed = OUT.read_text(encoding="utf-8")
    if committed == text:
        print("OK  %s" % OUT)
        print("    已提交的台账与按当前参数重新生成的结果**逐字节一致**（%d 行，sha256=%s）"
              % (len(rows), sha256(text)))
        print("    --check 是非破坏模式：**没有写任何文件**。")
        return 0
    a = committed.splitlines()
    b = text.splitlines()
    diffs = []
    for idx in range(max(len(a), len(b))):
        x = a[idx] if idx < len(a) else None
        y = b[idx] if idx < len(b) else None
        if x != y:
            diffs.append((idx + 1, x, y))
    print("MISMATCH  %s" % OUT)
    print("    已提交：%d 行，sha256=%s" % (len(a), sha256(committed)))
    print("    重新生成：%d 行，sha256=%s" % (len(b), sha256(text)))
    print("    不同的行：%d 处" % len(diffs))
    for lineno, x, y in diffs[:5]:
        print("      第 %d 行" % lineno)
        print("        已提交：%s" % ((x or "<无此行>")[:160]))
        print("        重新生成：%s" % ((y or "<无此行>")[:160]))
    if len(diffs) > 5:
        print("      ...（其余 %d 处未逐条打印）" % (len(diffs) - 5))
    print("    已提交的台账**未被改动**。用 --out <path> 可以把重新生成的结果落到别处做人工比对。")
    return 1


def main(argv):
    try:
        check, out, run, run8 = parse_args(argv)
    except ValueError as exc:
        print("拒绝执行：%s" % exc)
        print("未写入任何文件。")
        print(USAGE)
        return 2

    # Validate BEFORE building or writing anything: a bad run id must not leave
    # a half-written ledger behind.
    for label, run_id in (("run id（位置参数 1）", run), ("run id（位置参数 2）", run8)):
        problem = validate_run_id(run_id, label)
        if problem:
            print("拒绝执行：%s" % problem)
            print("未写入任何文件。")
            print(USAGE)
            return 2

    rows = build_rows(run, run8)
    text = render(rows)

    if check:
        return do_check(rows, text)

    target = pathlib.Path(out) if out else OUT
    if out and target.resolve() == OUT.resolve():
        print("注意：--out 指向的正是已提交的台账路径 %s（等同于默认模式）。" % OUT)
    if target.parent and not target.parent.exists():
        target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text, encoding="utf-8")

    kinds = kinds_of(rows)
    print("wrote %d rows (scope=%d) kinds=%s" % (len(rows), len(scope), kinds))
    print("  写入路径：%s" % target)
    try:
        print("  写入路径（相对仓库根）：%s" % target.resolve().relative_to(REPO))
    except ValueError:
        pass
    print("  本次 run id：%s（%d 行引用）" % (run, sum(1 for r in rows if r.get("evidence_run") == run)))
    print("  本次 run id：%s（%d 行引用）" % (run8, sum(1 for r in rows if r.get("evidence_run") == run8)))
    print("  写入内容 sha256：%s" % sha256(text))
    if out:
        print("  已提交的台账 %s **未被改动**（--out 模式）。" % OUT)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
