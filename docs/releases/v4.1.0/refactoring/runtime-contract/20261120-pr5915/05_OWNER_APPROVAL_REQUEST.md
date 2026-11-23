# Owner审批请求

source_agent=Codex GPT-6.1 Sol；source_run=RUNTIME-5915-20261120；timestamp=2026-11-20（Asia/Shanghai）；evidence_hash=sha256:57fe60174b4642317a534de402027e42603ba976601a8ed1ff4916cb9fb7df2a（独立复跑元数据）。conflict_resolution=Owner批准前为PROPOSED技术裁决建议，不是实施授权或最终独立合格证。

请求审定的是ADR-011统一契约及其迁移/验收条件，**不请求本轮直接写核心代码或派发Worker**。

| 待定项 | 建议 | 未批准时 |
|---|---|---|
| 数据主账/恢复责任 | TxStore唯一物理mutation/undo；TM只协调状态 | 不删除现有某一层后即发布 |
| 稳定持久RowId | 新协议中snapshot/WAL记录共同RowId；编码/sidecar另审 | 不以临时slot冒充跨重启ID |
| 未提交状态 | NO-STEAL私有workspace，独立namespace/spool与预算 | 不允许flush把pending变committed |
| durability各模式 | Every/GroupCommit按durable frontier ACK；弱模式显式分级 | 不宣称全模式同步durable |
| 格式/旧数据信任 | 版本化snapshot/WAL、离线副本迁移；污染/缺日志拒绝自动恢复 | 不承诺零格式变化，不丢数据清表“修复” |
| 故障模型/资源 | clean reopen、process exit、SIGKILL、WAL replay、断电分开 | 不将未跑项升级为通过 |
| 隔离/功能范围 | 按现有产品承诺审定，bare File缺陷不外推所有装配 | 不擅降为Read Uncommitted/取消事务 |
| 独立验收 | 设计者与实施者之外的Reviewer+Owner | Codex不自签、M-Pre不自动关闭 |

INV-07b应保留P0数据正确性风险候选，正式表述为“事务打开时flush后正常重开可读到未提交行”；“崩溃恢复后已提交可见”的部分目前NOT_RUN。Owner批准后仅DSH按自身permit/资源/LOCAL_ONLY规则规划后续执行；本包不覆盖或撤销其他既有授权，也不新增实施授权。
