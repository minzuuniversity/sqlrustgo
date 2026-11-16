#!/usr/bin/env python3
"""Generator for docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl (P1-3 ledger).

Kept in the repo so the ledger is reproducible from a clean checkout:

    python3 audit/v4.1.0/gen_neg_ledger.py <evidence-run-id>

The generator asserts that every scope gate (stage_required union the
SOAK/PERF/COMPAT/TPCH/STABILITY/CRASH/UPGRADE/CHAOS/QPS/SYSBENCH names in
GATE_CATALOG) has exactly one row, so the ledger cannot silently shrink.
Not a gate: it never runs in ci_gate.sh and never changes the denominator.
"""
import json, pathlib, re, sys

REPO = pathlib.Path("/Volumes/workspace/dev/sqlrustgo/.worktrees/410")
CAT = REPO / "docs/releases/v4.1.0/GATE_CATALOG.jsonl"
OUT = REPO / "docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl"
RUN = sys.argv[1] if len(sys.argv) > 1 else "20261105-negfix-003"
ISSUE = "issue#5598"
NEG = "audit/v4.1.0/neg/"
LOG_FX = "audit/v4.1.0/runs/%s/logs/003-neg-fixtures.stdout" % RUN
LOG_ST = "audit/v4.1.0/runs/%s/logs/004-neg-selftests.stdout" % RUN
NAME = re.compile(r"SOAK|PERF|COMPAT|TPCH|STABILITY|CRASH|UPGRADE|CHAOS|QPS|SYSBENCH", re.I)

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
 "scripts/gate/check_anti_ignore_gate.sh": ("fx02-anti-ignore-registry-drift.sh", "static",
   "往 tests/*.rs 加一个真实 #[ignore] 而不登记进 tests/baseline/ignore_registry.json ⇒ registry drift 判据变红（实测 rc=1）"),
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
 "scripts/gate/check_alpha_v410.sh": ("composite", "删 docs/releases/v4.1.0/STAGE.yaml 后 composite 必须变红（其 entry 分量已由 fx08 证伪）；composite 实测 200s 未跑完，需要热构建泳道"),
 "scripts/gate/check_anti_fabrication.sh": ("cargo", "删掉一条降级声明的 run 引用，或把白名单里一条已知失败测试改成真实失败 ⇒ 必须变红；先要 cargo check 热 target"),
 "scripts/gate/check_arch3_no_bypass.sh": ("cargo", "新增一处绕过 ExecutionEngine 的 DML 调用 ⇒ 必须变红"),
 "scripts/gate/check_arch_invariants.sh": ("cargo", "摘掉一条 invariant 的实现 ⇒ 必须变红"),
 "scripts/gate/check_arch_sem_debt.sh": ("cargo", "把语义债表里一条 CLOSED 改回 ACTIVE 且无计划 ⇒ 必须变红"),
 "scripts/gate/check_backup_restore.sh": ("runtime", "让 backup→restore 往返丢行（截断备份）⇒ 必须变红；需要构建产物"),
 "scripts/gate/check_beta_gate.sh": ("composite", "把 BETA 覆盖率实测降到 COVERAGE_MIN_BETA 之下 ⇒ 必须变红"),
 "scripts/gate/check_coverage_v312.sh": ("cargo", "把某个 crate 的覆盖率表替换成低于阈值的值 ⇒ 必须变红；llvm-cov 实测 >720s 未跑完"),
 "scripts/gate/check_full_gate_verification.sh": ("cargo", "让它调用的某道子门禁必然变红（复用 fx04 的夹具文件），composite 必须报该子门禁 FAIL 而不是吞掉退出码"),
 "scripts/gate/check_g16_compatibility.sh": ("external", "把一个兼容性用例期望改错或断掉 oracle ⇒ 必须变红；需要外部 MySQL/Oracle"),
 "scripts/gate/check_g1_tpch_baseline.sh": ("runtime", "喂空结果集或篡改 SF1 基线数字 ⇒ 必须变红；需要 SF1 数据集与热 target"),
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
 "scripts/gate/check_sql_corpus_gate.sh": ("cargo", "截断语料库或篡改 expected 输出 ⇒ 必须变红"),
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

rows = []
for gate, (fx, cost, note) in sorted(MUT.items()):
    rows.append({"gate": gate, "stages": cat[gate].get("stages") or [], "class": cat[gate]["class"],
                 "kind": "mutation-verified", "fixture_id": fx[:-3], "fixture": NEG + fx,
                 "evidence_run": RUN, "evidence_log": LOG_FX, "cost": cost, "note": note})

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
             "evidence_run": RUN, "evidence_log": LOG_FX, "cost": "static",
             "note": "check_alpha_v410.sh 的 entry 分量：删 docs/releases/v4.1.0/STAGE.yaml ⇒ E1 判红、门禁 rc=1。该行是 scope 之外（class=ci_reachable）的加测，不等于 composite 本身已证伪"})

rows.append({"gate": "scripts/gate/check_stage.sh",
             "stages": [], "class": cat["scripts/gate/check_stage.sh"]["class"],
             "kind": "mutation-verified", "fixture_id": "fx14-stage-version-uninferable",
             "fixture": NEG + "fx14-stage-version-uninferable.sh",
             "evidence_run": RUN, "evidence_log": LOG_FX, "cost": "static",
             "note": "阶段驱动的版本解析：(A) 无可推输入必须 rc=2 且不得先评阶段；(B) 分支不含版本时必须解析 "
                     "v4.1.0（改前实测评到已归档 v3.9.0 并写日志进该目录）；(C) --version 指向不存在的发布线时"
                     "必须 rc=2 且不得创建 docs/releases/v9.9.9。反向变异：同一夹具打修复前 9f82eeb777 三臂全部 FIXTURE-FAIL"})

SELFSAB = {
 "scripts/gate/check_commit_message.sh": "fx11-commit-message-detector-crippled.sh",
 "scripts/gate/check_coverage_single_implementation.sh": "fx12-coverage-single-impl-detector-crippled.sh",
 "scripts/gate/check_gate_test_integrity.sh": "fx13-gate-test-integrity-detector-crippled.sh",
}

for gate, (impl, note) in sorted(SELF.items()):
    rows.append({"gate": gate, "stages": cat[gate].get("stages") or [], "class": cat[gate]["class"],
                 "kind": "selftest-verified", "selftest_impl": impl,
                 "selftest_neg_fixture": NEG + SELFSAB[gate],
                 "evidence_run": RUN, "evidence_log": LOG_ST, "cost": "static", "note": note})

for gate, (attempt, reason) in sorted(UNF.items()):
    rows.append({"gate": gate, "stages": cat[gate].get("stages") or [], "class": cat[gate]["class"],
                 "kind": "unfalsifiable", "reason": reason,
                 "falsification_attempt": NEG + attempt,
                 "evidence_run": RUN, "evidence_log": LOG_FX, "cost": "static"})

for gate, (cost, target) in sorted(PEND.items()):
    rows.append({"gate": gate, "stages": cat[gate].get("stages") or [], "class": cat[gate]["class"],
                 "kind": "pending-fixture", "target": target, "close_run": ISSUE, "cost": cost})

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

with open(OUT, "w", encoding="utf-8") as f:
    for r in rows:
        f.write(json.dumps(r, ensure_ascii=False, sort_keys=True) + "\n")
kinds = {}
for r in rows:
    kinds[r["kind"]] = kinds.get(r["kind"], 0) + 1
print("wrote %d rows (scope=%d) kinds=%s" % (len(rows), len(scope), kinds))
