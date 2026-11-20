## 关联 Issue

Refs #5235，Refs #5909。Doc / architecture preparation；没有Closes，不自动关闭Issue。

## 一、5-类文档

- SPEC：docs/releases/v4.1.0/refactoring/evidence/PR_SPEC.md <!-- section:SPEC -->
- TEST_PLAN：docs/releases/v4.1.0/refactoring/evidence/PR_TEST_PLAN.md <!-- section:TEST_PLAN -->
- TEST_DESIGN：docs/releases/v4.1.0/refactoring/evidence/PR_TEST_DESIGN.md <!-- section:TEST_DESIGN -->
- REVIEW：docs/releases/v4.1.0/refactoring/evidence/PR_REVIEW.md（PENDING独立审查） <!-- section:REVIEW -->
- ACCEPTANCE：docs/releases/v4.1.0/refactoring/evidence/PR_ACCEPTANCE.md（仅交付检查） <!-- section:ACCEPTANCE -->

## 变更与授权边界

4.1.0 release独立refactoring目录逐字节归档HARA九报告及全部56原件；新增13规划交付物、十份PROPOSED ADR、DSH十二件交接包。按真实DSH Schema生成18个C-ALPHA-01候选payload，额外依赖/LOCAL_ONLY/批准/预算约束哈希绑定。M1–M6仅锁定epic，75条旧TF输入不继承PASS。

任务授权=0、token授权=0，2.57M为建议总上限。LOCAL_FIRST、凭据/出口/子agent隔离、Schema外约束强制、TF分类/原文日志问题未闭合。M-Pre未关闭，M0正式接口实现未授权。DSH未来唯一日常执行控制器，Codex不另建调度器。

无正式源码、Cargo、阶段、tag、门禁、权限或服务改动；只推文档分支，不直写develop或自动合并。

## Test Plan与Evidence

本轮原件hash、十ADR摘要、18 native schema、DAG无环/无缺失、75来源case、审批边界及负向变异见docs/releases/v4.1.0/refactoring/evidence/validation.json。D8、PR sections、入口链接、whitespace及GitNexus staged检查原文单列。正式build/clippy/full cargo test NOT_RUN，不引用旧结果报新PASS，CI待平台执行与人审。

source_agent=Codex GPT-6.1 Sol；source_run=C-ALPHA-01-HANDOFF-20261120；timestamp=2026-11-20 Asia/Shanghai；evidence_hash见docs/refactor/v4.1.0-c/handoff/10_HANDOFF_MANIFEST.json及validation中的manifest digest；conflict_resolution=Owner/独立Reviewer优先，设计者不自签。

入口：docs/releases/v4.1.0/refactoring/README.md；交接：docs/refactor/v4.1.0-c/handoff/00_HANDOFF_README.md。

## 风险与审批

03 JSONL禁止裸导入live队列；无DSH/Hub/KOS live写入。Owner需另批DSH接管、预算、ADR/隔离/格式/资源及独立验收。Gate Integration：不新增、不修改、不启用正式门禁。
