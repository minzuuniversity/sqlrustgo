# 事务不变量登记（待审）

> source_agent: Codex GPT-6.1 Sol · source_run: C-ALPHA-01-HANDOFF-20261120 · timestamp: 2026-11-20 (Asia/Shanghai)
> evidence_hash: sha256:4649fccc566613f04568ef3564e9d585739c4e92c73ad5972714b7d4f65b992f（HARA 输入 manifest）；本轮来源及产物 hash 见 evidence/SOURCE_BINDINGS.json 和 handoff/10_HANDOFF_MANIFEST.json。
> conflict_resolution: Owner 裁决优先；本文 PROPOSED，计划交付不等于 ADR 批准、任务授权或产品认证。

| ID | 不变量 | HARA / 首批工作包 |
|---|---|---|
| INV-01 | rollback精确恢复原多重集、每个mutation只撤销一次 | F01，C01-05 |
| INV-02 | 相同行值独立RowId；UPDATE主键不改内部ID，迁移/复用遵守generation | F02，C01-05/14 |
| INV-03 | db/table/tx/epoch显式封闭；跨库同名表不串写 | R1，C01-12/14 |
| INV-04 | savepoint cursor仅属本tx，部分回滚不污染顶层恢复 | F01/F05，C01-14 |
| INV-05 | 一条语句多行/多表/trigger所有可逆变化共同原子提交/撤销 | F09/F10，C01-10/12 |
| INV-06 | undo失败不返回成功，不丢未完成cursor，不允许继续commit | F05，C01-08 |
| INV-07 | 未提交私有状态不能进入committed snapshot/recovery可见基线 | F03，C01-06 |
| INV-08 | 同步模式ACK前durable_lsn>=commit_lsn，两commit入口语义相同 | F04，C01-07 |
| INV-09 | durable决定后发布失败不是“确定未提交”；OutcomeUnknown需查询/幂等键 | F04，C01-07/18 |
| INV-10 | checkpoint证书基于fsync数据/目录的generation，不是flush调用布尔 | F08，C01-13 |
| INV-11 | reclaim保留active/replica/PITR需要日志；segment切换可恢复 | F08，C01-13/17 |
| INV-12 | Recovery失败不得进入Recovered，重试不能空成功 | F07，C01-08 |
| INV-13 | approved隔离语义由真实并发+版本/物理主账联合证明 | F03，C01-12/14 |
| INV-14 | capability不足写前拒绝；wrapper不凭空增加inner回滚能力 | F06，C01-09 |
| INV-15 | 索引/聚类/cache与同一committed RowStore发布/撤销一致 | F09/F11，C01-10/11 |
| INV-16 | 旧格式迁移、重复值redo幂等、备份/复制/PITR共享恢复身份 | F08，C01-17 |
| INV-17 | 必需case的FAIL/KNOWN_FAIL/NOT_RUN/未批准UNSUPPORTED均阻断认证 | TF，C01-04/16 |
| INV-18 | 每case/任务/调用回执可关联SHA、原文、全SHA256和预算；无云fallback | TF/LOCAL_ONLY，C01-02/03/16 |

F08是HYPOTHESIS：决定性测试可得到反例、受限安全或INCONCLUSIVE，不要求它预先RED。OLD_RED→NEW_GREEN→MUTATION_RED仅适用于已经跑出旧缺陷的具体装配；不能把所有wrapper强行规定为旧红。TRUNCATE等DDL按批准MySQL语义，不统一假定可回滚。

## 矩阵映射状态词表（invariant_mapping_status）

矩阵 `TEST_COVERAGE_MATRIX.json` 的每条 case 都带一个 `invariant_mapping_status`。
本词表是**封闭**的：矩阵里出现第三个取值即视为门禁失败（`verify_matrix.py` 的 M7 判红）。
工具里的常量 `MAPPING_STATUSES` 与本表必须一致，M7 每次运行都把本表读回来比对，工具与登记表不能各说各话。

| 取值 | 含义 | 何时用 | 伴随要求 |
|---|---|---|---|
| `MAPPED` | 该用例的断言对应至少一条已登记不变量 | `invariant_ids` 非空 | `invariant_mapping_basis` 必须点名它所主张的每一条 id；一条 id 被多个用例主张时，每个用例都要各自说明 |
| `NO_INVARIANT_MATCH` | 该用例的断言不对应任何已登记不变量 | `invariant_ids` 为空（`[]`） | 该用例必须登记在 `docs/refactor/v4.1.0-c/takeover/INVARIANT-GAP-PROPOSALS.md` 的索引表里 |

`MAPPED` 的判定必须来自**测试源码**：读 `source.test_name` 指向的那个测试函数，判定它**实际断言**了什么，
再在上表的 18 条里找**同一主体**的 id。**标签、声明目的、feature 归属都不能作为判据** ——
按分桶给整组贴同一组 id 正是本词表要防的缺陷（Task 7 的判定记录：75 条里 28 条共用一句「回滚后状态 == 事务前」，
其中混着 `test_classify_sql_transaction` 这类与回滚毫无关系的测试）。

源码读不到时（例如 bespoke 探针未迁入仓内），按 `NO_INVARIANT_MATCH` 处理并登记缺口提案，
**不得**靠标签或声明目的补一个 id 充数。
