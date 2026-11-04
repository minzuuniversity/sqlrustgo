# v4.1.0 open issue 台账（IL-1 产物）

- 生成：`scripts/gate/issue_ledger.py`（本文件是生成物，手改会被下次生成覆盖）
- 输入：Gitea `type=issues` open=106；分诊行=106（`expert_triage.py`）
- 证据等级（行级）：advisory-only 94 / cross-checked 12；ladder credited lineage=gpt,qwen；实际分诊席位血统=gpt,qwen-strata
- 规则：ladder 有两个 credited 血统 ≠ 本条被两个血统分别核对过。只有同一 issue 被 >=2 个 family 分诊才升级 cross-checked
- 分诊只回答：域 / 破坏了哪条声称 / 可否自动化 / 修复形状 / 疑似重复。**不回答"是否已修好"**，销账仍走 PR 关联与关闭验证

## 0. 证据字段（AFP 5 字段，ADR-014）

| 字段 | 值 |
|---|---|
| source_agent | mcode-macmini |
| source_run | 20261105-triage-001 |
| timestamp | 2026-11-04T20:59:01Z |
| evidence_hash | sha256:dbae2af2baa6a53d03d888c9b6a1d7a60daf29494803d6808e76ef10b7f2c254 |
| conflict_resolution | 单席位行一律 advisory-only；同一 issue 被 >=2 family 分诊才升 cross-checked（本次 12 行升级） |

输入指纹（可用 `shasum -a 256` 复现 evidence_hash）：

- `ISSUE_TRIAGE_2026-11-05.jsonl` = `sha256:80afd329fb8b0c89ec600413d688f5410dcd3d94c186a671af6396b8c4a04503`
- `ISSUE_TRIAGE_CROSSCHECK_2026-11-05.jsonl` = `sha256:9719244075474f3c50f1e8afa2f2674708434867bed4c59f139b6543ad6c6852`
- `all.json` = `sha256:b0d9d8acc8574af490b896a1a8efe0324c98b53ef56f531f505a5d83bb45a5ad`

复现输入快照（重新拉取得到新指纹，因 issue 状态会变）：

- issues：`python3 scripts/gate/gov_probe.py issues --dump <path>`（open issues 逐页 limit=50，dump 行自报 sha256）
- 分诊：`python3 scripts/gate/expert_triage.py --input <issues dump> --seat <seat> --out <jsonl> --batch N`（席位与延迟逐行记录在 jsonl）

## 1. 分布

| 维度 | 分布 |
|---|---|
| 优先级 | priority/p0 47 / priority/p1 43 / priority/(none) 14 / priority/p2 2 |
| 功能域 | test-validity 24 / ci-infra 18 / gate 12 / sql-semantics 9 / storage-wal-crash 8 / release-stage 7 / docs-claims 7 / tooling 5 / perf-tpch 5 / coverage 3 / vector-index 3 / conn-lifecycle 2 / security 2 / backup-restore 1 |
| 可自动化 | auto 37 / auto-needs-infra 32 / needs-human-decision 29 / manual-host 7 / infra-config 1 |
| 疑似重复 | 1 条 |

## 2. 台账

| # | 优先级 | 域 | 标题 | 破坏的声称 | 可自动化 | 修复形状 | blast | dup |
|---|---|---|---|---|---|---|---|---|
| [#5235](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5235) | p0 | release-stage | [IAIA-410] v4.1.0 实现真实性与系统集成正式审计 Master | v4.1.0 实现真实性与系统集成审计未完成 | human-decision | cross-module | large | - |
| [#5226](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5226) | p0 | sql-semantics | [IAIA-410/A04] SQL 语义与 MySQL 协议差分审计 | SQL语义与MySQL协议兼容 | auto-infra | cross-module | large | - |
| [#5228](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5228) | p0 | storage-wal-crash | [IAIA-410/A06] 事务、WAL、恢复与故障注入真实性审计 | 事务ACID与崩溃恢复语义正确 | auto-infra | cross-module | large | - |
| [#5223](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5223) | p0 | ci-infra | [IAIA-410/A01] 冻结审计基线与证据 manifest 契约 | 审计结果可复现且证据完整 | auto-infra | infra-config | medium | - |
| [#5227](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5227) | p0 | storage-wal-crash | [IAIA-410/A05] 索引、Buffer Pool 与持久化存储真实性审计 | 索引与存储引擎真实生效 | auto-infra | cross-module | large | - |
| [#5204](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5204) | p0 | tooling | [IAIA-410] AOCI-CODE 索引实测：治理在 HEAD 已阻塞且工具工作流无法解开；索引格式内嵌机器相关绝对路径 | 代码索引工具链可用且合规 | human-decision | infra-config | small | - |
| [#5233](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5233) | p0 | release-stage | [IAIA-410/A11] Independent Judge 证据复核与冲突裁决 | 独立复核与冲突裁决未完成，证据可信度存疑 | human-decision | cross-module | large | - |
| [#5234](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5234) | p0 | release-stage | [IAIA-410/A12] 最终审计报告、治理审批与阶段门禁衔接 | 最终审计报告与治理审批缺失，阻断晋级 | human-decision | cross-module | large | - |
| [#5520](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5520) | p0 | tooling | [已订正] 5 远端同步工具无法推进 gitea250 且可能静默 exit 0；明文凭据仍在 gitcode 的公开分支上可读 | 5远端同步工具无法推进gitea250且可能静默exit 0 | manual-host | infra-config | medium | - |
| [#5525](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5525) | p0 | backup-restore | [备份/恢复] 逻辑备份与恢复都是骨架：backup 枚举 0 张表、restore 只数 INSERT 行数 | 逻辑备份/恢复声称成功但实际未执行任何数据操作 | auto-infra | cross-module | large | - |
| [#5537](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5537) | p0 | gate | [门禁失效] v4.1.0 ALPHA 阶段 2 个门禁阻塞项：coverage 脚本漏传 feature（storage 覆盖率为 0）+ | v4.1.0 ALPHA门禁通过及覆盖率数据可信度 | auto | localized | medium | - |
| [#5582](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5582) | p0 | conn-lifecycle | [引擎] 空闲连接回收器从未真正生效 —— register_connection 只留 Weak，唯一强引用是临时值 ⇒ weak.upg | 空闲连接回收器从未真正关闭任何连接，导致TLS连接泄漏及文档误导。 | human-decision | cross-module | medium | - |
| [#5363](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5363) | p0 | ci-infra | [CI] act_runner 未注册：50/50 个 workflow run 卡在 queued，零完成（自 2026-10-15T13 | CI 完全停摆，所有 PR 未经 CI 验证 | manual-host | infra-config | large | - |
| [#5375](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5375) | p0 | ci-infra | test job 编译失败：partition_e2e_test 缺 clustered 字段；另有 storage --lib 29 个既 | CI测试套件的可编译性契约 | auto | localized | medium | - |
| [#5377](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5377) | p0 | storage-wal-crash | create_index 没有 _in_db 变体：命名库的索引既建不了也查不到 | 命名数据库索引创建与查询的一致性契约 | auto | cross-module | large | - |
| [#5381](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5381) | p0 | security | 仓库内硬编码可用的 Gitea 明文凭据（token 嵌入 git push URL、curl -u、文档） | 仓库代码不包含敏感凭据的安全契约 | human-decision | cross-module | large | - |
| [#5403](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5403) | p0 | gate | [门禁失效] postcheck 另有 3 个必失败步骤：调用已废弃的 check_coverage.sh、v3.7.0 证据绑定 74 项 | postcheck门禁步骤应正确执行且不破坏受控文件 | auto-infra | infra-config | large | - |
| [#5412](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5412) | p0 | perf-tpch | [引擎] 带 PRIMARY KEY 的逐行 INSERT 仍是 O(N^2) —— pk_lookup_cache 被它自己要加速的那次  | 带主键表逐行INSERT应为O(N)而非O(N^2) | auto | cross-module | large | - |
| [#5415](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5415) | p0 | perf-tpch | [wire 路径] oracle_g15_tpch_sf01 的 Q22 在 server 崩溃后 36µs 内报 Broken pipe  | TPCH Q22查询在Wire协议下稳定执行 | manual-host | cross-module | large | - |
| [#5461](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5461) | p0 | gate | [GA 流程] 证据 §7.25.7 把 3 项 GA 门禁标注为「本机未验证（**CI 承接**）」，实测 **CI 也从不调用它们**  | GA门禁清单中特定检查项在CI中的实际执行覆盖 | auto | localized | medium | - |
| [#5463](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5463) | p0 | sql-semantics | [引擎] GROUP BY 引用未知标识符时不报错，执行器行宽不匹配 panic ⇒ 整个 server 挂掉 | SQL语义校验完整性及服务器稳定性契约 | auto | localized | large | - |
| [#5466](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5466) | p0 | test-validity | [GA 证据] A04 差分矩阵已入库证据**内部自相矛盾**且**无法从 clean checkout 复现** —— 同一 case 的 | GA 证据链完整性与可复现性契约失效 | human-decision | infra-config | medium | #5389 |
| [#5491](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5491) | p0 | gate | [门禁] GA-2 的 SOAK 判据只检查「脚手架文件在不在」；`run_ga_soak.sh` 与 `extract_soak_repo | GA-2 声称通过 SOAK 测试，实际仅检查文件存在且无执行脚本 | auto-infra | cross-module | medium | - |
| [#5509](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5509) | p0 | storage-wal-crash | [引擎] FileStorage 的 DDL 列名匹配整体大小写敏感 —— DROP/MODIFY/SET DEFAULT 都只认「存储时的 | 文件后端与内存后端DDL行为一致性及MySQL兼容语义 | human-decision | cross-module | medium | - |
| [#5514](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5514) | p0 | conn-lifecycle | [引擎] 30 秒空闲连接回收器把「客户端在算长任务」当成「连接已死」⇒ 合法场景被断连；应改为心跳/更长超时 | 空闲连接回收机制误杀长思考客户端，且无禁用开关 | human-decision | cross-module | medium | - |
| [#5117](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5117) | p0 | release-stage | [v4.1.0][总控] Alpha→Beta 门禁整改与晋级审核 | Alpha至Beta晋级门禁满足 | human-decision | cross-module | large | - |
| [#5361](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5361) | p0 | sql-semantics | [A04] 拒绝 ON 子句含表达式的合法 LEFT JOIN（sqlite 正常返回） | 拒绝含表达式的合法 LEFT JOIN，偏离 SQL 标准 | auto | cross-module | large | - |
| [#5362](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5362) | p0 | tooling | [toolchain] develop 上 cargo clippy -D warnings 失败：fault_injection.rs u | cargo clippy --all-features -D warnings 失败 | auto | one-line | medium | - |
| [#5364](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5364) | p0 | ci-infra | [CI][evidence-graph] ingest_git_commit 对短 commit hash panic —— evidenc | evidence-graph-gate 因 panic 在每个 run 失败 | auto | localized | medium | - |
| [#5366](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5366) | p0 | ci-infra | [CI] evidence-graph-gate 结构上无法 PASS：SHA 未回退 + 边早于节点 + 缺 VERIFIED_BY 边 | evidence-graph-gate 无法产出 PASS，证据链断裂 | auto-infra | cross-module | medium | - |
| [#5368](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5368) | p0 | ci-infra | [CI] evidence-graph-gate 解析不了自己的 PASS：grep 模式与 pretty-print JSON 不匹配 | 证据链门禁对PASS状态的解析契约 | auto | one-line | small | - |
| [#5388](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5388) | p0 | coverage | [CI] fail-fast 掩盖了 382 个从未执行的测试目标：首次全量跑暴露出与 TPC-H 无关的失败，需归因分流 | 测试覆盖率与CI门禁有效性 | auto | infra-config | large | - |
| [#5392](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5392) | p0 | storage-wal-crash | 引擎缺陷：文件存储后端在首列同值的行上塌缩，LOAD DATA 报告成功但查询只能看到约 1/4 的行 | 文件存储数据完整性与持久化 | auto | cross-module | large | - |
| [#5393](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5393) | p0 | perf-tpch | [wire 路径] file storage 后端上 TPC-H Q9/Q18 资源耗尽致服务器 OOM 被杀、Q19 解析失败 | 文件存储后端查询执行能力 | auto | cross-module | large | - |
| [#5394](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5394) | p0 | sql-semantics | [引擎] LEFT OUTER JOIN 的 ON 条件过滤会丢弃左表行（TPC-H Q13 少算 71 个客户） | LEFT OUTER JOIN标准SQL语义 | auto | cross-module | large | - |
| [#5400](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5400) | p0 | gate | [门禁失效] check_correctness_tests_production_storage.sh 的 R2/R4 断言在 #5317 | 生产存储默认路径检查应准确反映当前代码结构 | auto | localized | medium | - |
| [#5402](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5402) | p0 | gate | [治理缺口] CI postcheck job 在 600 次历史 run 中从未成功执行过一次 —— 其内含的发布门禁与 6 个硬阻塞 m | postcheck发布门禁应能成功执行以验证合规性 | auto-infra | infra-config | large | - |
| [#5409](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5409) | p0 | sql-semantics | [引擎] INSERT 的列名列表不被校验：未知列名静默写入 NULL 行，不报 ER_BAD_FIELD_ERROR(1054) | INSERT未知列名应报ER_BAD_FIELD_ERROR而非静默NULL | auto | localized | large | - |
| [#5427](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5427) | p0 | test-validity | [测试非自包含] tpch_compliance_test 依赖仓库里不存在的 `data/tpch-tiny/`（8 个 .tbl + 预 | TPCH合规测试数据在仓库中可用 | auto-infra | new-fixture | medium | - |
| [#5428](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5428) | p0 | test-validity | [测试非自包含] quick_query 依赖机器本地的绝对路径 `/tmp/tpch-sf1`（含 WAL 文件）⇒ 任何干净环境必然失败 | 测试环境自包含且不依赖本地状态 | auto-infra | new-fixture | medium | - |
| [#5436](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5436) | p0 | release-stage | [GA 流程] `STAGE_CONFIG.yaml` 的 GA `branch_pattern` 不含 `develop/*`，而 v4. | GA流程定义覆盖当前开发分支 | human-decision | infra-config | large | - |
| [#5452](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5452) | p0 | gate | [分支保护] `STAGE_CONFIG.yaml` 的 `GA.merge_policy`（3 个 reviewer + 4 类审批 +  | 分支保护与合并门禁策略已生效 | auto-infra | infra-config | large | - |
| [#5454](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5454) | p0 | release-stage | [治理流程] 版本自动化的 release-please 链路在仓库里**不存在**，同时规范又把**所有** PR 标题锁死为 `fix( | 版本自动化链路(release-please)存在性及PR标题规范一致性 | human-decision | cross-module | medium | - |
| [#5462](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5462) | p0 | sql-semantics | [引擎] GROUP BY 引用 SELECT 列表别名时被当成字符串常量（所有行被并成一组） | GROUP BY子句对SELECT列表别名解析的正确性 | auto | cross-module | large | - |
| [#5507](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5507) | p0 | storage-wal-crash | [存储/性能] 重构 MemoryStorage 的「提交快照」机制：让逐行 INSERT 的提交代价回到 O(1)（#5412 的根因修复 | 逐行 INSERT 提交代价应为 O(1)，实际为 O(N²) | auto-infra | cross-module | large | - |
| [#5527](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5527) | p0 | sql-semantics | [LOAD DATA] 不检查重复键：有主键的表重复导入静默追加，与 MySQL 的 Duplicate entry 不一致 | LOAD DATA 在有主键/唯一索引时未执行重复键检查，违反 MySQL 兼容性契约 | auto-infra | cross-module | medium | - |
| [#5528](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5528) | p0 | ci-infra | [CI] runner 换成 Alpine/musl 镜像后缺静态 zlib ⇒ 所有需要链接测试二进制的 job 都红 | CI 门禁与测试流程无法通过，阻塞所有 PR 合并 | auto-infra | infra-config | large | - |
| [#5531](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5531) | p1 | docs-claims | 多 Agent 并行协作台账（sqlrustgo / v4.1.0）：身份登记 + 文件归属矩阵 + 认领台账 | 多Agent并行协作缺乏可操作的身份与状态协调机制 | human-decision | localized | small | - |
| [#5225](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5225) | p1 | test-validity | [IAIA-410/A03] 静态生产调用链与假集成审计 | 生产代码路径无死代码或假集成 | manual-host | cross-module | medium | - |
| [#5229](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5229) | p1 | test-validity | [IAIA-410/A07] 测试系统反向审计与 mutation REVIEW 清零 | 测试套件能有效发现生产缺陷 | auto-infra | new-fixture | medium | - |
| [#5231](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5231) | p1 | perf-tpch | [IAIA-410/A09] v3.12、v4.0、v4.1 跨版本正确性与性能回归 | 跨版本性能与正确性无回归 | auto-infra | infra-config | large | - |
| [#5232](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5232) | p1 | security | [IAIA-410/A10] 认证、协议、会话与隔离边界审计 | 认证、协议与会话隔离边界审计未完成 | auto-infra | cross-module | large | - |
| [#5458](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5458) | p1 | docs-claims | [治理流程] `PR_TEMPLATE.md` 在仓库里**不存在**，但 `AGENTS.md` 与本 PR 自己的 OpenSpec 任 | PR模板文件存在性及AGENTS.md中流程约束的可执行性 | manual-host | localized | small | - |
| [#5384](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5384) | p1 | vector-index | cosine_similarity_simd 用「自己到自己的距离」当范数 ⇒ 相似度恒为 0，cosine 距离恒为 1.0 | 余弦相似度计算的正确性契约 | auto | localized | large | - |
| [#5397](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5397) | p1 | ci-infra | [CI 基础设施] runner 拉取 actions/checkout 间歇性失败（unexpected EOF），PR 流水线被基础设施 | CI流水线稳定性与可复现性 | auto-infra | infra-config | medium | - |
| [#5423](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5423) | p1 | test-validity | [测试 flaky] 根包 lib 测试 `test_v410_atomic_parallel_degree_getter_clamps_t | 单元测试在并发环境下稳定通过 | auto | localized | small | - |
| [#5114](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5114) | p1 | test-validity | [v4.1.0][P1] Alpha→Beta：BustubX-EDU 与多连接模型回归矩阵 | 多连接模型与并发隔离的正确性保证 | auto-infra | new-fixture | large | - |
| [#5115](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5115) | p1 | test-validity | [v4.1.0][P1] Alpha→Beta：SOAK 恢复、故障注入与 1h/24h/168h 阶梯 | SOAK稳定性验证的可审计性与状态真实性 | auto-infra | infra-config | large | - |
| [#5376](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5376) | p1 | test-validity | 崩溃测试 harness：SIGKILL 落在边界之后 —— 崩溃测试没有证明它声称证明的事 | 崩溃恢复测试对WAL提交边界的验证有效性 | auto-infra | new-fixture | medium | - |
| [#5380](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5380) | p1 | ci-infra | 5remotes_sync.sh 在 ssh-agent 失效时无限挂起：此前「SSH 远端不可达」的结论是错的 | 多远端同步脚本的健壮性契约 | auto-infra | localized | medium | - |
| [#5383](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5383) | p1 | ci-infra | test_run_soak_loop_tier3.sh 在 macOS 上假失败：wc -l 输出带前导空格，断言用 == 硬比较 | 跨平台测试脚本的稳定性契约 | auto | one-line | small | - |
| [#5385](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5385) | p1 | vector-index | FlatIndex::search 的 cosine 分支返回距离却按降序排 ⇒ take(k) 保留下来的是 k 个最不相关的向量 | 向量检索Top-K结果的相关性排序契约 | auto | localized | medium | - |
| [#5389](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5389) | p1 | test-validity | [Test] a04_mysql_diff_matrix_test 跑一次就改写仓库里 81 个证据文件，污染工作区 | 测试环境隔离与无副作用 | auto | localized | small | - |
| [#5453](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5453) | p1 | docs-claims | [治理流程] 提交信息规范**三份并存、互相不一致**，且没有任何机器裁决（无 commit-msg 钩子、CI 无校验、近 30 个 co | 提交信息规范存在且被强制执行 | human-decision | infra-config | small | - |
| [#5473](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5473) | p1 | gate | [治理] P14 的基线文件 `tests/baseline/drift_baseline.json` 的 `total_findings: | P14元门禁基线数据不可信，导致防退化判据失效 | human-decision | cross-module | medium | - |
| [#5474](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5474) | p1 | gate | [门禁] P14 的 Check 3「grep on cargo test stdout without PIPESTATUS check」 | 门禁脚本能精确识别未校验退出码的管道命令 | auto | localized | small | - |
| [#5116](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5116) | p1 | perf-tpch | [v4.1.0][P1] Alpha→Beta：性能基线与自动回归门禁 | 性能基线自动回归门禁生效 | auto-infra | infra-config | medium | - |
| [#5360](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5360) | p1 | sql-semantics | [A04] 未加别名表达式/聚合列名暴露 Rust Debug AST（协议层正常，CLI 层丢失） | CLI 层暴露 Rust Debug AST 而非列名 | auto | localized | small | - |
| [#5386](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5386) | p1 | vector-index | InvertedIndex::search_with_limit 截断 HashSet 迭代 ⇒ 返回随机子集，同 query 两次调用结果 | 检索结果确定性与Top-K相关性排序契约 | auto | localized | medium | - |
| [#5387](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5387) | p1 | gate | [Scale Layer] TPC-H SF=1 数据在 CI 内不可得：dbgen 未 vendor + GitHub 出口被封，ADR- | G4规模层门禁的可执行性 | human-decision | new-fixture | medium | - |
| [#5390](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5390) | p1 | tooling | [Repo] .gitignore 残留未解决的合并冲突标记（151/375 行），约 224 行忽略规则语义不确定 | 仓库忽略规则的正确性 | auto | one-line | small | - |
| [#5391](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5391) | p1 | test-validity | [Test] tests/integration/oracle/ 下仍有 19 个零断言文件（只有 eprintln），查询全错也会通过 | 测试断言有效性 | auto | localized | medium | - |
| [#5395](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5395) | p1 | test-validity | [测试判定口径] eval_22_vs_sf01 只比对行数，漏掉 Q13 这类「行数相同、内容不同」的分歧 | 测试判定口径的一致性 | auto | localized | medium | - |
| [#5398](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5398) | p1 | test-validity | g13_oltp1_concurrent_select_test 的测量无判别力且约 50% 误报 —— 需按「连续量 + 与核数无关 +  | G13并发SELECT测试应能可靠检测读锁回归 | auto | localized | small | - |
| [#5405](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5405) | p1 | test-validity | sqlrustgo_cli_soak_e2e_test 找的二进制名 sqlrustgo-cli 在当前布局里不存在（重命名前的旧包名）—— | soak e2e测试应能找到正确的CLI二进制入口 | human-decision | localized | small | - |
| [#5407](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5407) | p1 | ci-infra | 「CI 变红了」这个说法本身没有证据：test job 在全部 400 次 run 里零次成功（它从来没绿过） | CI test job应能成功执行并反映真实状态 | human-decision | infra-config | large | - |
| [#5408](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5408) | p1 | ci-infra | [CI 基础设施] `timeout … | tee` 杀不掉挂死的测试二进制 —— CI step 没有上界（run 8031 超时后仍  | CI测试步骤应有超时上限且能正常终止 | auto-infra | infra-config | medium | - |
| [#5411](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5411) | p1 | test-validity | [测试 flaky] insert_db_isolation_5057 的三个用例共用同一临时目录、并行时互相删除 —— 基线对照实测 1. | INSERT/UPDATE/DELETE库隔离测试应稳定通过 | auto | localized | small | - |
| [#5429](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5429) | p1 | test-validity | [测试环境依赖] tpch_full_test 的 MySQL / PostgreSQL 用例需要 CI 未提供的外部数据库服务 ⇒ 3 个 | TPCH测试在CI环境中可独立运行 | auto-infra | infra-config | medium | - |
| [#5434](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5434) | p1 | test-validity | [测试基础设施] regression_test 的 test_regression_suite 是跨目标的 meta 套件，它把别的测试目 | 测试失败归属准确且版本一致 | human-decision | cross-module | medium | - |
| [#5450](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5450) | p1 | docs-claims | [发布流程文档] `RELEASE_GATE_CHECKLIST.md` 规定的 4 个发布门禁脚本 **3 个不存在、1 个从不被调用** | 发布门禁脚本存在且可执行 | human-decision | cross-module | medium | - |
| [#5459](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5459) | p1 | docs-claims | [治理流程] `docs/governance/PR_STAGE_CONTROL_FRAMEWORK.md` 是一份**未合并 PR 的正文 | 治理文档PR_STAGE_CONTROL_FRAMEWORK的有效性与引用状态 | manual-host | localized | small | - |
| [#5460](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5460) | p1 | docs-claims | [治理流程] `openspec/config.yaml` 声明 `schema: spec-driven`，但 **197 个变更里只有  | Spec-driven开发流程声明与变更目录结构校验的一致性 | auto | new-fixture | medium | - |
| [#5478](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5478) | p1 | test-validity | [测试非自包含] `BinaryTableStorage` 的故障注入是**进程级**的（4 处 `maybe_delay()` 读进程 e | 故障注入测试可并行执行且互不干扰 | auto-infra | cross-module | medium | - |
| [#5490](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5490) | p1 | sql-semantics | [测试清单] `KILL CONNECTION <id>` 对**不存在的连接**返回成功 + 一行文本 —— 假成功 | KILL CONNECTION对未知ID应报错而非假成功 | human-decision | localized | medium | - |
| [#5494](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5494) | p1 | tooling | [生产缺陷] `sqlrustgo` CLI wrapper 定位 `sqlrustgo-mysql-server` 只看 cwd 与 PA | CLI wrapper 在任意目录下定位 server 二进制的能力 | auto | localized | medium | - |
| [#5495](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5495) | p1 | test-validity | [测试层] `tests/compat/*.sh` 6 个脚本此前「找不到二进制 ⇒ SKIP ⇒ exit 0」⇒ 假绿；本批已改，但缺少 | 声称存在有效的回归测试覆盖，实际为未执行的死代码 | human-decision | infra-config | medium | - |
| [#5498](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5498) | p1 | test-validity | [测试判别力] `q4_nested_and_short_circuit_test` 的哨兵计数器（`CONJUNCT_*_EVALS` / | Q4测试声称验证短路语义，实际仅验证结果正确性 | human-decision | cross-module | small | - |
| [#5503](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5503) | p1 | test-validity | [可见性] NOT-RUN 披露在**本地**完全不可见 —— `cargo test` 捕获了通过测试的 stdout | NOT-RUN三态披露在本地不可见，导致本地绿与CI绿语义不一致 | auto-infra | localized | medium | - |
| [#5526](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5526) | p1 | ci-infra | [CI] runner 镜像不固定 clippy 版本：同一 label 相隔一小时跑出 1.95.0 与 1.91.0 两种 lint 集 | CI构建可复现性及Lint基线门禁稳定性 | auto-infra | infra-config | medium | - |
| [#5583](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5583) | p2 | coverage | [门禁] .github/workflows/coverage-parallel.yml 是第三份覆盖率判据：自带测量 + 自抽 line_ | 覆盖率判据唯一性契约被破坏，存在独立且冲突的阈值判定 | human-decision | infra-config | medium | - |
| [#5502](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5502) | p2 | docs-claims | [文档债] v4.1.0 发布线有 27 条失效的仓库路径指针；全仓扫描约 2300 条 —— 逐条清账 | v4.1.0发布线文档中的仓库路径指针均指向真实存在的文件 | human-decision | localized | medium | - |
| [#5569](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5569) | - | ci-infra | [CI 基础设施红] hp-z6g4 的 gmp-platform-runner.service 带 Requires=docker.ser | CI 基础设施稳定性与隔离性契约 | manual-host | infra-config | large | - |
| [#5548](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5548) | - | test-validity | [测试失效] wrapper「不能委托时必须诚实失败」的契约在 CI 里从未真正被验证（前提造不出来） | 找不到 server 时 wrapper 必须诚实失败 | auto | localized | small | - |
| [#5549](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5549) | - | test-validity | [测试 flaky] g13 并发测试的前提是「这台机器有多快」而不是「代码对不对」 | 并发查询并行执行能力的测试有效性 | auto-infra | localized | small | - |
| [#5556](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5556) | - | ci-infra | [测试挂死] fault-injection 的 reach() 每个 WAL 追加自旋 120s ⇒ cargo test -p sqlr | CI 测试套件在预算内完成且无孤儿进程残留 | auto-infra | cross-module | large | - |
| [#5567](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5567) | - | ci-infra | [CI 红 · 既有] test-crates job 在 TPC-H fixture 生成阶段红，且该 step 没有 NOT RUN 兜 | CI 环境阻塞应记为 NOT RUN 而非代码红 | human-decision | infra-config | medium | - |
| [#5573](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5573) | - | ci-infra | [CI 基础设施红] act_runner 每 6–9 分钟被要求优雅关机 ⇒ 在飞 job 容器被 pause ⇒ 5 分钟宽限到点 jo | CI长任务执行稳定性及Job超时/中断处理机制 | infra-config | infra-config | large | - |
| [#5577](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5577) | - | test-validity | [测试副作用] cargo test --workspace 会覆写已提交的证据文件（a04-5226 的 21 个对照输出被抹成占位符） | 集成测试隔离性及已提交证据文件的不可变性 | auto | localized | medium | - |
| [#5579](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5579) | - | coverage | [门禁] RC/GA 门禁的覆盖率算法与 coverage 门禁不是一份实现 ⇒ storage 被静默跳过 ⇒ GA 的 A5「67%」是 | GA门禁覆盖率阈值判定逻辑及storage模块覆盖统计 | auto | localized | medium | - |
| [#5580](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5580) | - | gate | [门禁] check_rc_ga_gate.sh 的 ga / rc / beta 模式永远判 FAIL（期望 D1/D2 却从不执行）⇒  | GA/RC/Beta 晋级门禁可执行且能通过 | auto | localized | large | - |
| [#5581](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5581) | - | gate | [门禁] 覆盖率阈值有 5 份实现（SSOT 50/75/80/85 vs 脚本里的 75/80/47/42/50）、测量有 6 份实现 ⇒ | 覆盖率门禁阈值与测量口径的单一事实来源（SSOT）一致性 | human-decision | cross-module | medium | - |
| [#5551](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5551) | - | ci-infra | [基础设施红] docker 地址池被占满 ⇒ CI job 在「Start image」这一步全灭，一个测试都没跑 | CI 基础设施可用性：确保测试套件能正常启动执行 | auto-infra | infra-config | large | - |
| [#5557](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5557) | - | storage-wal-crash | [运维阻塞] server 在 --wal-sync every 大量写入后卡进不可中断 I/O（进程状态 U）⇒ SIGTERM 不生效、 | SIGTERM/SIGKILL 可预期终止服务，不受 I/O 阻塞影响 | human-decision | cross-module | medium | - |
| [#5584](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5584) | - | storage-wal-crash | [测试/恢复] 崩溃测试从来没有在它声称的边界上崩溃：`after_commit_fsync` 只在 autocommit 提交上触发，且在 | 文件存储路径下COMMIT边界未执行fsync，崩溃测试断言失效且索引恢复导致表不可读 | human-decision | cross-module | large | - |
| [#5592](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5592) | - | release-stage | [看板] v4.1.0 · ALPHA 阶段总览（2026-11-04，2026-11-05 刷新） | v4.1.0 ALPHA阶段状态与进度追踪的SSOT一致性 | auto | localized | small | - |
