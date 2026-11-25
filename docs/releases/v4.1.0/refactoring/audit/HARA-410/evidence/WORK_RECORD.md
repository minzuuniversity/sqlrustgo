# HARA-410 文档与证据工作记录

source_agent=Codex GPT-6.1 Sol；source_run=HARA-410-20261120；timestamp=2026-11-20（Asia/Shanghai）；conflict_resolution=用户本轮只读审计/禁止提交推送的指令优先，未执行DOC_CHECK流程中的远端提交步骤。证据hash见MANIFEST.json及各报告头部。

范围：在专门detached worktree新增九份审计报告与离线反例、证据；没有纠正/重写旧治理文档。原工作树既有.gitignore修改保留，后续AGENTS/AOCI并发变化不归因、不回退。

## 计划、执行、复核

1. 发现/基线：读取用户两份附件、AGENTS与7+1治理文件，实跑Phase0/D8，查询远端SHA、分页Issue/PR/tag/release；发现工作树分支不同、Cargo版本入口漂移、历史release元数据不一致。
2. 文档计划：按用户指定顺序，先02/03/04/05的独立源码与Git证据，再06/07/08方案，读ARR系列后写09，最后01。九文件是新增事实/建议，不修改旧记录；CHECKLIST为元数据、链接、hash、集合完整性、运行事实和写入范围。
3. 执行：使用apply_patch写报告/审计harness；在冻结源码和历史detached副本构建；每次命令完整stdout/stderr、argv、timestamp、exit保存到JSON。没有降低正式断言或门禁；没有正式重构。
4. 检查：verify_delivery.py检查九报告、五证据字段、内部链接、hash绑定、P0/P1双向差集/无重复、关键probe实际输出及wire结果。初次delivery-check.json暴露检查器误把API的pull_request:null当PR；核对104条均null后改为检查非null对象，保留初次失败，重跑记录delivery-check2.json。只声明检查范围，不把机械文档一致性当内核通过。
5. 工作报告：01是裁决入口，09是未闭合证据/UNREAD/NOT-RUN清单，MANIFEST.json记录所有交付物和关键源码hash。D8输出只说明登记状态，不说明数据库正确。
6. Gate：本轮仅Phase0规定D8实跑；没有修改governance、判据、badge，因此未执行治理改动P11–P16或阶段认证全套。不以未执行项声称通过。
7. Git：只在独立审计worktree保留新增报告/证据；按文档可审查要求可暂存，不commit/push/PR/merge，不动阶段/tag/Issue/权限/SOAK。

## 交付状态

报告交付、缺口明确；完整产品验证INCOMPLETE。原型/报告作者PASS与Codex本轮实测分开；NOT-RUN不计通过。历史harness使用单独Cargo.lock与当前toolchain，非原发布依赖环境重演。编译产物target-hara410*留在审计工作树，未纳入报告或正式源码。

## 恢复与重现

无需回退正式代码；审计文件没有改写旧资产。保留三个detached审计/历史工作树与证据用于复核，不自动删除。重跑证据命令均以主审计工作树为cwd；查看JSON的argv和对应historical manifest路径。MANIFEST自身不递归hash自己，其他交付物逐文件SHA256可复算。
