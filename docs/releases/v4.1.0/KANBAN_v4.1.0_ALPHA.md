# v4.1.0 · ALPHA 阶段看板

- 版本：`v4.1.0`　分支：`develop/v4.1.0`　看板建立时 HEAD：`b5039acb1c`
- 阶段状态 SSOT：`docs/releases/v4.1.0/STAGE.yaml`（`current_stage: "ALPHA"`）——本看板不复制它，只引用它
- 看板制度：`docs/governance/STAGE_KANBAN.md`　机读台账：`KANBAN_v4.1.0_ALPHA.jsonl`　看板 ISSUE：**#5592**
- 建立：2026-11-04，`mcode-macmini`
<!-- LEDGER:BEGIN (generated from KANBAN_v4.1.0_ALPHA.jsonl — 不要手改本段) -->

- 台账计数：未开始 18 / 待复核 3 / 完成 11 / 阻塞 5 / 合计 37

| ID | 任务 | 状态 | 验收判据（可证伪） | 证据 | 阻塞方 |
|---|---|---|---|---|---|
| KB-1 | 建立阶段看板制度 + 本看板 + 看板 ISSUE | 完成 | 制度文档存在且五节齐备；看板文档+JSONL+ISSUE 三者 ID 集合一致 | docs/governance/STAGE_KANBAN.md; 本看板; Gitea ISSUE #5592 (openclaw/sqlrustgo) | - |
| EV-1 | 把 2026-11-04 治理基线探针归档，使看板每个数字可回溯 | 完成 | manifest.yaml 含 sha256 与 git_head；文档引用的数字均指向该 run | audit/v4.1.0/runs/20261104-govprobe-001（manifest.yaml + logs/001..004，audit_run.py verify OK） | - |
| EX-1 | 咨询专家梯：席位注册表 + fail-closed 聚合(R1/R2/R3) + 证据契约 | 待复核 | 夹具对 R1/R2/R3 各做反向变异后结论必须翻转；证据含 MANIFEST+sha256 | scripts/gate/test_expert_ladder.py（夹具 2026-11-04 22:3x 实测 exit 0）; audit/v4.1.0/runs/20261104-seats-001/logs/001-seats-health.stdout | - |
| EX-2 | R4 席位标定：席位须答对 ground-truth 探针才可参与裁定 | 完成 | 答错席位即使连通也不计入血统数；不标定时口径必须写明 reachability only | scripts/gate/test_expert_ladder.py（exit 0）; audit/v4.1.0/runs/20261104-seats-001（flash-next-252 标定 4/4=1.00 calibrated；health 退出码 2=不可裁定） | - |
| EX-3 | 恢复≥2 个可裁定血统（deepseek 登录 / chatgpt 稳定性） | 完成 | health 的 lineages_credited ≥2 且 adjudication_capable=true | audit/v4.1.0/runs/20261104-seats-002（health 退出码 0：credited lineage=[gpt,qwen]，adjudication_capable=true；chatgpt-web 标定 4/4、lat 68.4s；flash-next-252 标定 3/4、lat 101.1s）; 对照 run 20261104-seats-001（同一命令当时 credited=[qwen]，capable=false） | - |
| IL-1 | 106 个 open issue 台账化并分诊（域/破坏的声称/可自动化/修复形状） | 完成 | 每条 open issue 都有分诊行（域/破坏的声称/可自动化/修复形状/blast/dup）；台账含 AFP 5 字段与输入指纹；缺行必须 rc!=0 | docs/releases/v4.1.0/ISSUE_LEDGER_2026-11-05.{md,jsonl}（106 行 / 0 缺分诊 / 12 cross-checked / 94 advisory-only）; ISSUE_TRIAGE_2026-11-05.jsonl(106, sha256:80afd329) + ISSUE_TRIAGE_CROSSCHECK_2026-11-05.jsonl(12, sha256:97192440); run 20261105-triage-001（frozen e409de89bf，8 命令 exit 0，evidence_hash sha256:a2538ba853…）里的 008-ledger-verify；校验器 audit/v4.1.0/verify_ledger.py L1-L4 rc=0，反向变异 G(门限2->1)/H(指纹篡改)/I(谎报 cross-checked) 均 rc=1；快照边界见 IL-4 | - |
| P0-1 | gate/registry.py + gate_catalog.jsonl + check_registry.py：168 个 check_* 逐个登记 | 待复核 | 反向变异：从 registry 删一条登记 ⇒ 元门禁必须红；孤儿数 83→0 | SSOT 登记表 docs/releases/v4.1.0/GATE_CATALOG.jsonl(168 行=check_n 口径)；核对器 scripts/gate/gate_registry.py(不叫 check_*、不进阶段清单，AGENTS 分母不变)；夹具 scripts/gate/test_gate_registry.py 9 条反向变异全部被抓(M1 删一行/M2 class 谎报/M3 抹 reason/M4 can_fail 谎报/M5 抹 stages/M6 幽灵登记/M7 抹 disposition/M8 编造 evidence/M9 处置与执行点矛盾)；MX-1 的显式调用声明设计随 #5594 走 owner 授权 | - |
| P0-2 | run_gates.py + gate_report.py + results.jsonl + 三态 | 未开始 | 缺一项/重复一项/退出码丢失 三个夹具都必须判 INCOMPLETE | - | - |
| P0-3 | 打开合并拦截（分支保护状态检查+审批）+ merge_blocking.py | 阻塞 | 把 enable_status_check 关掉 ⇒ 该门禁必须红；API 不通 ⇒ 输出必须 INCOMPLETE 而非 PASS | 实测：enable_status_check=false、required_approvals=0（develop/v4.0.0 为 1） | owner（分支保护属不可协商项，需人工授权） |
| P0-4 | 回收 62 个无人引用的死门禁（逐个给 reason 或退役）—— 修引用口径后从 37/36 涨到 62，见附录 A | 待复核 | gate_catalog.jsonl 中 dead_list 每条有 reason；计数 37→0；不与 P0-1 同 PR | 83/83(62 dead + 21 orphan_referenced)逐条给 disposition+note+逐字 evidence；分布 archive-candidate 33 / needs-execution-point 25 / needs-external-host 12 / keep-rewire 12 / superseded 1；机器判据 gate_registry C6(不交代=rc1；理由在门禁文件里找不到=rc1 反编造)+C7(已挂执行点却标归档=rc1)；退役/删除属门禁清单变更⇒已立项 owner 决策件 #5595(含 34 条归档候选全名+证据)；12 条 keep-rewire 不归档的理由挂在台账 open P0 上 | P0-1 |
| P0-5 | 幽灵执行点处置：13 个 GitHub workflow 永不运行 + 三份 168h-soak 配置归一 | 未开始 | 可执行性扫描根集合在 registry 显式声明；保留的 .github 必须标『仅存档不执行』 | gh_wf=13 gt_wf=8，见 audit/v4.1.0/runs/20261104-govprobe-001/logs/001-gov-metrics.stdout | P0-1 |
| P1-1 | ssot_coverage.py：21 条 exit_criteria 全部映射到门禁或 manual_only | 未开始 | 故意漏一条映射 ⇒ 必须红 | stage_map=0 见 audit/v4.1.0/runs/20261104-govprobe-001/logs/001-gov-metrics.stdout | P0-1 |
| P1-2 | capability_claims.py：真 CLI+真 MySQL+真落库+崩溃重启读回+备份恢复逐值比对 | 未开始 | 把生产存储换成 MemoryStorage ⇒ 必须红；只 Mock 的测试不得支撑『已验证』声明 | - | - |
| P1-3 | 负向夹具扩面：GA/RC/BETA 清单 34 道 + 名字含 SOAK/PERF/COMPAT/TPCH 全部 | 未开始 | fixtures.jsonl 对清单内门禁 100% 覆盖；未覆盖必须显式 UNFALSIFIABLE(reason) | 168 道里仅 23 道有反例，见 audit/v4.1.0/runs/20261104-govprobe-001/logs/001-gov-metrics.stdout H.neg | P0-1 |
| P1-4 | 每道门禁写 effect + honesty.py 核对声明与夹具一致 | 未开始 | 声称能抓 X 但没有 X 的夹具 ⇒ 红 | honesty.json 见 audit/v4.1.0/runs/20261104-govprobe-001 | P1-3 |
| P1-5 | evidence_inventory.py：§1.4 刷新后的处数 PASS 声明分级绑定 run | 未开始 | 未绑定的新声明 ⇒ 红 | claims_hits=1123 claims_ref=0（22:05 快照 1098） 见 audit/v4.1.0/runs/20261104-govprobe-001/logs/001-gov-metrics.stdout | - |
| P2-1 | 文档事实面：doc_stamp/doc_freshness/doc_path_claim_check/gen_doc_index 移植接入 | 未开始 | 悬空路径 0；生成物『写盘口径』==『核对口径』（墙钟时间不得进比对域） | - | - |
| P2-2 | 目录策略机器化（顶层 45 个 .md、bench/benches/benchmarks 等归位） | 未开始 | DIRECTORY_POLICY.md 已有规则，缺执行点；新增违规 ⇒ 红 | topmd=45 见 audit/v4.1.0/runs/20261104-govprobe-001/logs/001-gov-metrics.stdout | - |
| ING-1 | 继承缺口 V400-09：168h SOAK 无 FINAL_REPORT | 阻塞 | 存在带 run id 的 FINAL_REPORT；否则不得销账 | docs/releases/v4.1.0/STAGE.yaml known_gaps.V400_09_soak | owner（长时程运行需要机器排期） |
| P0-6 | 修复 3 个已提交 audit run 的证据断链（缺日志/缺 meta/manifest_sha256=None），使 check_audit_manifest.sh 在干净检出上可绿 | 未开始 | 7 个已登记 run 的 auditmanifest 全部 exit_code=0；缺日志的要么补齐重跑 verify，要么带理由退役 | audit/v4.1.0/runs/20261104-govprobe-001/logs/003-gov-audit-manifest.stdout（iaia410-5112 / iaia410-a03 / iaia410-a05 exit_code=2） | - |
| EX-4 | 把治理度量做成可重放命令 gov_probe.py（metrics/honesty/auditmanifest/issues） | 完成 | 同一 HEAD 上重复运行，关键计数与归档快照逐项相等；漂移必须可解释 | scripts/gate/gov_probe.py; audit/v4.1.0/runs/20261104-govprobe-001（4 条命令 exit 0，verify OK；168/83/37/4/21/0 与快照相等） | - |
| P0-7 | 消除 fail-open 懒初始化全局：gate_executability.ALL_REQUIRED_SCRIPT_PATHS 未 build 即为空集，调用方无感 | 未开始 | 未调用 build_matrix() 时读取该全局必须抛错/返回哨兵；夹具证明漏调会被抓到 | 实测对照：漏调 ⇒ orphan 95 / dead 44；调用后 ⇒ orphan 83 / dead 37（与快照一致） | - |
| KR-1 | 看板台账改为 JSONL 单一来源：kanban_render.py（render/check/counts）+ 夹具 | 完成 | check 在干净板上 exit 0 且 render 幂等；夹具含反向变异（删一行 JSONL ⇒ check 必须红，恢复 ⇒ 必须绿） | scripts/gate/test_kanban_render.py（2026-11-04 实测 exit 0，9 项全 PASS）; audit/v4.1.0/runs/20261104-kanban-001 | - |
| KR-2 | 把 kanban_render check 接到既有门禁/CI 执行点（挂靠既有门禁，不新增分母） | 未开始 | 从 JSONL 删一行 ⇒ 被接到的既有门禁判红；未接线时看板漂移无人发现（本次已实测发生过） | 待生成：接线后的 Gitea Actions run 链接 | P0-2（需要三态 runner 才能把结论如实上报） |
| EX-5 | 恢复 deepseek 席位（桥 HTTP 500，上游 40003 invalid token，需人工登录） | 阻塞 | health 里 deepseek-chat ok=true 且 lineages_credited 含 deepseek（第三系佐证） | audit/v4.1.0/runs/20261104-seats-002/logs/001-seats-health.stdout（deepseek ok=false lat 0.58s） | owner（登录只能人工完成，Agent 不得代做凭据动作） |
| EX-6 | 查 flash-next-252 标定掉到 3/4 的那道题（席位可信度边界，影响它能否参与裁定） | 未开始 | 复现失败项并给出原因；若低于 calibration_min 则 health 必须不再计入该血统 | 对照：run 20261104-seats-001 标定 4/4 vs run 20261104-seats-002 标定 3/4（同一席位同一命令） | - |
| CI-1 | 刷新 CI 吞吐口径：看板旧行「3911 runs / 70 success」指不回任何可重放测量（实测 runs=1358、jobs=3495） | 完成 | gov_probe.py 新增 ci 子命令（只读、stdout JSONL）；test_gov_probe.py 钉死 conclusion 口径；看板 §6 改为实测值并指向归档 run | scripts/gate/gov_probe.py cmd_ci + summarize_runs；夹具 scripts/gate/test_gov_probe.py（未变异 exit 0；变异 A「成功判定改回 status」exit 1、变异 B「优先级过滤改回裸 P0」exit 1）；归档 run 20261105-cifact-001（本 PR 下一提交归档） | - |
| DR-1 | 整改计划基线刷新：死门禁 37/36 均为低估（探针把注释里的门禁名当调用），修探针后实测 62；docs410 与 claims 命中同步刷新 | 完成 | §1 每张表加「测量时间 + frozen_commit + 归档 run」列；与重测差异写进 §9；重跑 gov_probe metrics 后文档数字全部可指回 run | GOVERNANCE_REMEDIATION_PLAN_2026-11-04.md §1.4 + 附录 A（62 条全名）；修 scripts/gate/gov_probe.py 的 strip_prose()/gate_refs_in()；夹具 scripts/gate/test_gov_probe.py P1(文档字符串不计引用)/P2(真调用必须计)，反向变异 /tmp/srg-mut-e(strip 直通) 与 /tmp/srg-mut-f(只去 docstring) 均 rc=1 | - |
| LB-1 | 标签口径补齐：106 条 open 里 14 条无优先级、15 条无成本、12 条无 cap ⇒ 分层估算对约 12–15% 的 issue 不可核对 | 未开始 | 重跑 gov_probe issues 的 issue_summary：priority/cost/cap 的 (none) 全部归 0，或逐条显式记 NOT_APPLICABLE 理由 | audit run 20261105-cifact-001 的 census 行（priority/cost/cap 分布） | - |
| EX-7 | 查 37 条已分诊记录里 3 条 triage_error 的成因（席位抖动 / 输出不可解析 / 超时） | 完成 | 给出 3 条的原始输出指纹 + 复现命令；若属席位抖动 ⇒ 必须计入 expert ladder 的 health 失败，而不是在分诊侧静默跳过 | 3 条 triage_error(#5573/#5577/#5579) 在批量模式重跑后全部 ok（0 abstain）。根因：单条模式 max_tokens=300 + body 截断 2600 下输出不可解析。修 scripts/gate/expert_triage.py 的 parse_batch_output（缺报进 missing、范围外进 extra 且丢弃、围栏可恢复、不可解析不编造）；夹具 scripts/gate/test_expert_triage.py 反向变异 C(extra 不报)/D(missing 置空) 均 rc=1，未变异 rc=0 | - |
| IL-2 | 台账 cross-checked 覆盖率只有 12/106(11%)：47 条 P0 需做双血统复核才能升等（当前只有 qwen 血统跑批量，gpt 只复核 12 条） | 未开始 | ledger 输出 graded_crosschecked >= 47 且这 47 行 triage_families 含 >=2 family；credited_lineages<2 时仍一律 advisory-only | - | EX-5 或 EX-3 需恢复第二个可批量血统 |
| IL-3 | 疑似重复对 #5466 -> #5389 需人工确认是否合并（分诊只提示 dup，不裁定） | 未开始 | 人工裁定后：合并或标注 non-dup；台账重生成后 dup 数为 0 或写明保留理由 | - | 人工裁定（AGENTS: Issue 关闭须走 ISSUE_CLOSING_VERIFICATION + PR 关联） |
| EX-8 | 席位吞吐从单条改为批量：72 条 8 批跑完（每 10 条 22–101s），单条模式 39s/条 ⇒ 约省 5–8 倍席位轮次 | 完成 | expert_triage.py --batch N 输出仍是每 issue 一行，且漏答/范围外编号可被机器核对（夹具 C/D 反向变异红） | docs/releases/v4.1.0/ISSUE_TRIAGE_2026-11-05.jsonl（每行含 triaged_by/latency/prompt_sha256/output_sha256）; scripts/gate/test_expert_triage.py | - |
| MX-1 | 生产门禁引用图 gate_executability.gate_calls() 扫描未去注释的正文 ⇒ 注释里的『参见 xxx.sh』会把死门禁判活。探针已修，生产侧未修（改门禁判据需 owner 授权） | 阻塞 | gate_calls() 去掉注释与文档字符串后，死门禁数与 gov_probe metrics 一致（当前 62）；反向变异：恢复直扫 ⇒ 夹具必须红 | 已立项 Gitea openclaw/sqlrustgo#5594；探针侧已修 scripts/gate/gov_probe.py strip_prose/gate_refs_in + test_gov_probe.py P1/P2（反向变异 /tmp/srg-mut-e、/tmp/srg-mut-f 均 rc=1）；生产侧 scripts/gate/gate_executability.py:gate_calls 未改（等 owner 授权） | owner 授权修改清单内门禁的判据来源（AGENTS §8-7）；与 P0-1 gate_registry.jsonl 合并最省 |
| IL-4 | 台账快照边界：2026-11-04T20:59Z 的输入快照是 106 条，2026-11-05 run 重放得到 107 条（多出的 1 条是快照之后新建的 #5594=MX-1）⇒ 台账需用 107 条快照重生成并给 #5594 补分诊行 | 未开始 | 重生成后 rows==triaged==当期 open issue 数（verify_ledger L1），且 #5594 出现在台账；文档里的 open issue 计数与当期快照一致 | - | - |
| GS-1 | 同一判据三份实现：check_principles.sh(合并入口)无执行点，被它头部标 DEPRECATED 的 check_10_principles_v310.sh 反而在 CI 可达，check_5_principles_v310.sh 已死 ⇒ 实际跑的是被宣告废弃的那份 | 阻塞 | owner 在 #5595 裁定保留哪一份；裁定后：被弃者从 CI/清单摘掉(独立 PR，owner 做)，保留者必须有执行点；gate_registry C7 必须仍 rc=0 | GATE_CATALOG.jsonl 三行 + check_principles.sh 头部原文『both DEPRECATED as standalone but still informative』；已写入 #5595 §C | owner 裁定 #5595（门禁清单变更，AGENTS 红线） |
| KR-3 | 把 gate_registry.py check 与 test_gate_registry.py 接到既有 CI 执行点（挂靠既有门禁/既有 job，不新增门禁分母） | 未开始 | 既有 job 日志里出现 gate_registry 的 OK 行；登记表漂了 ⇒ CI 红；不得为此新增 check_* | - | KR-2 同类：需要既有 job 的改动窗口（P0-2 的三态 runner 一并挂） |
<!-- LEDGER:END -->

状态词汇只有五种（`未开始/进行中/待复核/完成/阻塞`）。"完成"= 做完**且**证据已归档；
门禁结论另按三态写 `PASS/FAIL/INCOMPLETE`，两者不互相借用。

---

## 3. 问题汇总（现象 / 实测根因 / 影响 / 谁能解）

| # | 现象 | 实测根因 | 影响 | 谁能解 |
|---|---|---|---|---|
| Q1 | 门禁很多，抓不住问题 | 168 道里 145 道没有反例夹具；`nofail=4` 道结构上永不红、`both_bad=4` 道既无反例又吞退出码 | 任何"全绿"都不能推出"没缺陷" | Agent（P0-1 / P1-3） |
| Q2 | 红了也照样合 | 状态检查关闭 + 审批 0；`swallow=56` 道把失败退化成成功 | 门禁不是闸门，是装饰 | Agent 建拦截 + **owner 开保护**（P0-3） |
| Q3 | 文档里 1123 处 PASS 声明（22:05 快照 1098，此后新增 2 份文档使计数上移），0 个总闸 | 声明与 run 不绑定（`claims_ref=0`，`claims/` 与 `INDEX.yaml` 无核查器） | "报告好看、一用就崩"的直接机制 | Agent（P1-5） |
| Q4 | 13 个 GitHub workflow 永不运行 | 本仓 CI 在 Gitea Actions（8 个 `.gitea/workflows`），`.github/workflows` 是历史包袱 | 判据存在但无执行点；文档引用其结论即幽灵证据 | Agent（P0-5） |
| Q5 | 21 条阶段标准 0 条落到机器 | `STAGE_CONFIG.yaml` 的 `exit_criteria` 无人映射 | 阶段跃迁靠叙述 | Agent（P1-1） |
| Q6 | 咨询专家梯曾**不具备裁定能力**，现已具备但仍有缺口 | 复核 run `20261104-seats-002`：`credited lineage=[gpt,qwen]`、`adjudication_capable=true`（chatgpt-web 标定 4/4、flash-next-252 标定 3/4）。缺口：`deepseek-chat` 仍 HTTP 500（上游 `40003 invalid token`，需人工登录）；`local-qwen35-9b` 与 `gmp35b-250` 不 ok（前者 120s 无有效返回，后者空 content+finish=length）。注意 252:8001、252:8002、macmini:8002 同源，只有 qwen 一系时不算双系 | 双系裁定现已可用（gpt+qwen），但第三系缺失，且 flash-next 标定掉到 3/4 需查（EX-6） | owner 登录 DeepSeek（EX-5）；Agent 查标定掉分（EX-6） |
| Q7 | 继承缺口未闭：`V400-09` 168h SOAK 无 FINAL_REPORT；3 道 alpha 门禁自 v4.0.0 起持续 FAIL | 见 `STAGE.yaml` 头部注释与 `known_gaps.V400_09_soak`（2026-10-07 已证伪过一次"文件存在"式销账） | ALPHA 出口条件之一长期挂空 | owner 排期长时程运行 |
| Q8 | 106 个 issue 无人认领、无里程碑 | 创建后无分诊；标签以 `state/ready`(82) 为主 | 无法排期，问题面不可见 | Agent（IL-1） |
| Q9 | 证据链门禁**在已提交树上就是红的** | `check_audit_manifest.sh` 扫描 7 个已登记 audit run，其中 `iaia410-5112-evidence-20261015`（缺 `commands/001.stdout.log`）、`iaia410-a03-*`（`manifest_sha256=None`）、`iaia410-a05-*`（缺 `commands/1.meta.yaml`）verify 退出码 2 | 已提交的审计证据不可重放；`IAIA_410_SSOT` 要求的"verdict 可从 manifest+日志重放"不成立 | Agent（P0-6） |
| Q10 | 度量工具本身 fail-open | `gate_executability.ALL_REQUIRED_SCRIPT_PATHS` 是 `build_matrix()` 里懒填的全局，未调用即空集，调用方拿不到任何错误：本次实测漏调使 orphan 从 83 虚高到 95、dead 从 37 到 44 | "0 条清单"会被读成"没有门禁要求"，与 AFP §4「不得默认成功」冲突 | Agent（P0-7） |
| Q11 | 门禁要求可重放，但 `*.json`/`*.log` 被 `check_no_log_tbl_json.sh` + `.gitignore` 拒绝入仓 | 两条规则本身不冲突，缺的是**兼容的证据文件类型约定**（`.stdout`/`.stderr`/`.jsonl`/`.yaml` + legacy run 布局） | 若沿用 `evidence/<kind>/MANIFEST.yaml` + `.json`，提交即触发文件治理门禁 | 已在 STAGE_KANBAN §2 定口径 |
| Q12 | `audit_run.py scaffold` 把 frozen_commit 钉在 `audit/v4.1.0/commit.txt`（A01 的 bf48818c，2026-10-09） | 冻结指针是 IAIA-410 的 SSOT，移动它属人工决定 | 在 develop 尖端做新度量只能自行声明 frozen_commit | **owner**（是否重冻由人决定） |
| Q13 | 看板 Markdown 与 JSONL 台账**实测已经漂移**（IL-1 任务文本写 105、JSONL 写 106；§6 证据索引引用了不存在的 `MANIFEST.yaml`/`metrics.json`） | 台账此前是手写的两份副本，没有生成关系；`*.json` 禁入仓又让索引沿用了旧命名 | "看板好看、任务对不上"正是本制度要防的事；引用不存在的证据文件＝幽灵证据 | 已修：`scripts/gate/kanban_render.py` 把 MD 台账变为 JSONL 生成物，`check` 可判漂移（夹具 exit 0，含删行反向变异）；接线待做（KR-2） |
| Q14 | 同一个 open issue 数，三种口径给出 105 / 106 / 107 | Gitea `/issues` 默认**把 PR 混进来**且不带稳定排序：`type=issues`=106、`type=pulls`=1、不带 type=107；早期快照 105 是 #5592 建之前 | 看板与整改计划的数字口径不一致，读者会以为任务面在增长 | 已定口径：一律 `type=issues` + 按 number 去重（`gov_probe.py issues` 已是 106，与本节一致） |
| Q15 | `audit_run.py run` 会核对 HEAD 与 manifest 声明的 `frozen_commit`，提交一次就拒绝再跑 | 工具按"证据属于被冻结的树"设计，而治理度量是对当前树/当前环境的重复测量 | 每提交一次，新 run 必须显式声明新的 frozen_commit，否则无法归档任何新证据 | 已按 schema_version 1 显式声明并在 notes 说明；是否重冻 A01 指针仍是 owner 决定（Q12） |

---

## 4. 下一步（第一项必须是能产生新证据的动作）

1. **P0-1 起步**：提交 `gate/registry.py` + `gate_catalog.jsonl` 骨架，把 168 道 `check_*`
   逐个登记（进清单 / 退役带理由 / 归档带理由），配反向变异夹具（从 registry 删一条必须红）。
2. **IL-1 批量分诊收尾**（进行中，37/106 已分诊）：给 `expert_triage.py` 加 `--batch 10`，
   跑完剩余 69 条；台账按 `advisory-only` / `cross-checked` 两级标注，未交叉核对的一律不写成裁定。
   分诊产物必须先入库（`ISSUE_TRIAGE_2026-11-05.jsonl`）才算证据。
3. **请 owner 做两件事**：(a) 在已打开的 Chrome 窗口完成 DeepSeek 登录（上游已明确 `40003`）；
   (b) 授权打开 `develop/v4.1.0` 的状态检查与审批（分支保护属不可协商项，Agent 不得自行改）。
4. **P0-6 修证据断链**：`check_audit_manifest.sh` 在**当前已提交树**上就是红的（3 个 run verify 退出码 2）。先补齐/重跑这 3 个 run 或带理由退役，否则任何"证据可重放"的声明都不成立。
5. **P0-7 消除 fail-open 全局**：`gate_executability.ALL_REQUIRED_SCRIPT_PATHS` 未初始化即空集，会让"0 条清单"被读成"没有要求"。
6. **P0-2**：`run_gates.py` + `results.jsonl` + 三态，把"SKIP 当 PASS"从机制上切断。
7. 之后按 P0-4 → P0-5 → P1-1 → P1-3 顺序推进，每步一个 PR，每个 PR 带反向变异证据。
8. **KR-2 接线**：把 `kanban_render.py check` 挂到既有门禁执行点，让"看板漂移"当场变红而不是靠人工发现。

---

## 5. 看板 ISSUE

| 文档 | Gitea ISSUE | 关系 |
|---|---|---|
| `docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md` | [#5592 `[看板] v4.1.0 · ALPHA 阶段总览（2026-11-04）`](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5592) | ISSUE 是本文档的展示面；权威副本是文档 |

关闭规则见 `docs/governance/STAGE_KANBAN.md` §5：只有 ALPHA→BETA 跃迁获批后，
逐条补齐完成说明与证据、未完成项显式迁移到 BETA 看板，才允许关闭本 ISSUE。

**历史缺口（如实记录）**：v4.1.0 的 DRAFT 阶段没有看板（本制度 2026-11-04 才建立），
因此 DRAFT 的任务台账无法回溯核对。不追溯伪造补建；若需要，将按当前 HEAD 重建一份
"事后重建"看板并标注 `rebuilt_after_the_fact`。

---

## 6. 证据索引

| 数字/结论 | 归档 run |
|---|---|
| 168 / 23 / 83 / 37 / 21 / 1123 / 13 / 45 / 106 等全部基线数字 | `audit/v4.1.0/runs/20261104-govprobe-001`（`manifest.yaml` + `logs/001-gov-metrics.stdout`、`002-gov-honesty.stdout`、`003-gov-audit-manifest.stdout`、`004-gov-issue-census.stdout`） |
| 席位能力（当前）：credited lineage = 2（gpt + qwen），`adjudication_capable=true`；chatgpt-web 标定 4/4、flash-next-252 标定 3/4；deepseek 仍 ok=false | `audit/v4.1.0/runs/20261104-seats-002`（取代 seats-001 的单席位结论） |
| R1–R4 规则夹具与反向变异 | `scripts/gate/test_expert_ladder.py`（2026-11-04 实测 exit 0） |
| CI 真实终态：1358 runs / 39 success（2.87%）、failure 1109、cancelled 209；时长 median 853s / p90 2606s；jobs 分母 3495 | `audit/v4.1.0/runs/20261105-cifact-001`（`logs/002-gov-ci.stdout`）。**勘误**：本行原写「3911 runs / 70 success」，无法指回任何可重放测量（实测 neither runs=1358 也 jobs=3495），已按探针重测替换 |
