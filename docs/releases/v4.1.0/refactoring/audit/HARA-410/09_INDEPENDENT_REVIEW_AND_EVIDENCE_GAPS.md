# HARA-410 · 独立复核与证据缺口

source_agent=Codex GPT-6.1 Sol；source_run=HARA-410-20261120；timestamp=2026-11-20（Asia/Shanghai）；evidence_hash=sha256:aace53350866e985d52b5ea6101673965f3405c9df02cc9a0574bc20b2f21bb8（七份ARR/M-Pre原件hash清单）；conflict_resolution=先保存02–05独立源码调查，再阅读他方报告；分歧按本轮源码/运行证据处理，架构由属主裁决。

## 已读材料与独立性

`/Volumes/workspace/dev/srg-handover-p2a/` 中7份指定ARR/M_PRE2文件全部找到并实读。P2A-00/A/B/C/D/E/F、P2B、P2C0、P2C05、P2C1、P2C1b、P2C1c、P2C1_CONVERGENCE、P2C1_S3、P2C1_RAII、P2C1_ID、P2C1_ID2 报告也已读。原型分支源码、每轮原始cargo日志及反向变异未逐一独立重跑；那些PASS属于报告作者的声明，状态 **UNVERIFIED**。没有把其计数并入本轮VERIFIED-RUN。

独立调查使用e9ff21fca3；GitNexus用cd8索引作导航，再读冻结源码。未编辑他方原型、PR或分支。模型身份按本会话提供的GPT-6.1 Sol标注，未伪称已从桌面设置核验推理档位。

## 重要结论逐项复核

| 他方结论 | 判定 | 独立依据/差异 |
|---|---|---|
| 双层undo导致DELETE恢复重复行 | AGREE | F01同基线SQL实跑，两行而非一行 |
| slot/op_id/loc应分开 | AGREE（设计） | FileStorage row_idx、buffer值及MVCC next_row_identity事实支持；原型ID完整性未验 |
| Engine按值幂等恢复可作为最终策略 | DISAGREE | 无主键重复值不能由相等性区分身份；不能让第二层补偿代替唯一所有者 |
| flush迁移后BufferedInsert失效 | AGREE | F02；v3.12.0同用例也复现，补上历史来源 |
| 原型U/E/RAII已足以排除C/D | DISAGREE | 只覆盖运行期部分行为；F03/F04/F07及持久化frontier超出原有结论 |
| 无Commit即可在恢复时整体丢弃，运行期undo是唯一防线 | DISAGREE | F03中incomplete_txns=1，但已落snapshot的未提交行重启可见。过滤redo不等于undo脏snapshot |
| 恢复层可原样复用 | PARTIAL | 可复用codec/分组思想；StatefulRecoveryEngine失败终态F07必须改，snapshot/redo去重协议也需审 |
| 所有COMMIT均不flush | DISAGREE | WalStorage &mut commit:1267 inner.flush；lockfree路径不相同。必须按入口分开 |
| Every模式等同已提交持久 | DISAGREE | F04 CountWal默认Every显示lockfree sync=0；物理断电丢失尚NOT-RUN |
| 只有with_wal_and_checkpoint工厂注入checkpoint | DISAGREE | 默认MySQL server装配 `lib.rs:7718` 也传Some(checkpoint_manager)；应查生产工厂而非根builder单处 |
| buffer_pool缺构造器、无法安全构造 | DISAGREE | FileStorage::new_with_bufferpool:1232公开；本轮F11实跑rollback后仍有行。未证明默认server启用此配置 |
| buffer_pool“直接落页”意味着持久化 | DISAGREE（若作此解读） | 路径构造内存Page，scan miss跳过，没有磁盘loader；page不是durable页证书 |
| 12实现、三假事务，其余纯转发 | DISAGREE/PARTIAL | 05枚举11公开实现+test helper+未导出columnar；BinaryV2是具体leaf且也空成功，不能列成继承inner的wrapper |
| FileStorage已有undo故可标TRANSACTIONAL | DISAGREE（作为已验证能力） | F01/F02/F03/F09/F10反例直接否证完整语义；可写“有事务代码”，不可认证TRANSACTIONAL |
| 不支持后端应写前拒绝 | AGREE（机制），PARTIAL（产品范围） | Unsupported必须区别IO failure；但已经承诺的功能不能由Agent直接删减 |
| 内存slot说明无需改任何磁盘格式 | PARTIAL | 只在NO-STEAL/重启不保留旧undo引用等前提下可行；F03表明现状不满足。row payload可保留不等于持久化协议不变 |
| “语句级串行化”可推导RC | DISAGREE；后续撤回认可 | 应先验证脏读/自写/跨表snapshot/并发冲突；代码有bind snapshot与inner merge，不能简化为锁即隔离 |
| P0=37，R1=9 | AGREE（采集时点） | 本轮独立API集合核对，P1另为51；不采用ARR初稿34/36算术 |
| 原型失败后可发布回退双层undo | DISAGREE；后续撤回认可 | F01已确定数据错误，旧路径仅离线比较，不能发布fallback |
| binlog/replication/PITR不在ALPHA gate可不读 | DISAGREE | 持久化/事务协议改变会影响其consumer watermark和重放；是否在gate不改变依赖事实 |
| RAII补偿可证明进程崩溃安全 | DISAGREE（若扩张解释） | Drop不会在process exit/SIGKILL/OOM abort执行；RAII报告自身已有诚实限制，可保留其局部试验 |
| 原型可立即进入M0 | DISAGREE | M-Pre本身仍有未实测出口和Reviewer/owner决策缺口；只建议C契约设计评审 |

## 本轮新增与初读遗漏

相对于指定报告正文，本轮新增：F03的脏snapshot+恢复过滤反例、F04的Every两入口sync观察、F07的失败恢复变空成功，以及v3.12历史复现；F08补出了snapshot未sync与WAL先清空再重写的精确路径，但断电后果仍HYPOTHESIS。

本轮初读只把BufferPool分支列为可选可达源码；读M-Pre后按其证据缺口补查公共构造器并实跑F11。REPLACE/ODKU的具体缺失记录点由ARR线索促使再次检查，run5/6独立复现了更具体的“rollback后空表”；这部分不能声称完全由初读独立发现。简单PK UPDATE及buffered delete-all本轮负对照正常，不代表他方反例错误：构造、flush时机、wrapper不同，仍需原条件重跑。

## 未闭合证据与最小下一试验

| 缺口 | 当前等级 | 最小决定性证据 |
|---|---|---|
| 默认file+parallel的完整wire事务/隔离 | 默认file既有wire目标6项VERIFIED-RUN，parallel/隔离矩阵NOT-RUN | 6项0失败/0忽略见wire-rollback-full.json；仍需真实两连接异常交错、版本链和物理主账联合断言，不能推广全矩阵 |
| 真正崩溃/断电durability | F03只有无Drop进程退出；断电NOT-RUN | SIGKILL父子handshake固定WAL-sync、snapshot与truncate边界；VM断电/故障设备另验证 |
| checkpoint/reclaim原子性 | VERIFIED-CODE风险，后果HYPOTHESIS | active peer事务+截断中断，恢复oracle核对已commit和未commit多重集 |
| 后端全部认证 | 枚举已做，运行PARTIAL | 六leaf+五wrapper的支持组合contract tests，含BinaryV2，不支持组合先拒绝 |
| MVCC/SSI | 部分源码已读，版本级/并发NOT-RUN | commit前冲突验证、跨表snapshot、自写可见、rollback/savepoint版本清理 |
| 全DML/Trigger/LOAD DATA/跨模型 | 路径矩阵PARTIAL，部分UNREAD | 所有承诺入口的副作用清单+第N步失败，父事务一并abort/rollback |
| 历史SQL、迁移、产物绑定 | 三版本共同SQL已跑：v3.11回滚后空表、v3.12在BEGIN中断（后续NOT-RUN）、当前重复行；迁移NOT-RUN，v3.11 release assets为空 | 需补完整共同SQL/恢复矩阵、tree/原发布二进制hash绑定、旧WAL/base/delta升级恢复；不能把v3.12最终保留1行算回滚正确 |
| 他方原型及mutation证明 | 报告已读，源码/运行UNVERIFIED | 冻结prototype commit，独立复跑原命令与反向变异，不能拿报告PASS互相背书 |
| CI/授权/阶段门禁 | 本轮只跑D8 | 只读服务器授权/required-check核对；原有阶段脚本在冻结提交实跑，由属主处理整改 |

## 属主需要决定

1. 接受C的实质边界：TxContext/RowId、统一commit/abort、committed-only snapshot、durable frontier和全部后端适配。
2. 确认已承诺隔离级别、支持后端/功能；不能把本报告的“待认证”自动改成产品“不支持”。
3. 是否允许版本化checkpoint/WAL sidecar，或要求先证明现有格式内可实现；涉及持久RowId/steal则需另评迁移。
4. 指定独立Reviewer与M-Pre新增恢复/并发出口，确认以后正式M0的授权范围。

当前意见：可以基于这些证据开展属主架构裁决；**不能据此立即合并原型、结束M-Pre或启动正式核心重构**。九份审计产物交付与所有产品验证完成是不同状态。UNREAD/NOT-RUN边界仍需后续实证，不能称全仓数据库正确性已认证。
