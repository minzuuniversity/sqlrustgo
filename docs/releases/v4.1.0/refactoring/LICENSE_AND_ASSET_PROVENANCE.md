# 测试资产来源与授权核查

> source_agent: Codex GPT-6.1 Sol · source_run: C-ALPHA-01-HANDOFF-20261120 · timestamp: 2026-11-20 (Asia/Shanghai)
> evidence_hash: sha256:4649fccc566613f04568ef3564e9d585739c4e92c73ad5972714b7d4f65b992f（HARA 输入 manifest）；本轮来源及产物 hash 见 evidence/SOURCE_BINDINGS.json 和 handoff/10_HANDOFF_MANIFEST.json。
> conflict_resolution: Owner 裁决优先；本文 PROPOSED，计划交付不等于 ADR 批准、任务授权或产品认证。

本轮仅归档自有HARA/SQLRustGo资产和本地DSH Schema，没有复制第三方corpus或TH3。SQLite官方版权页说明其交付代码/文档为公有领域，同时提醒部分构建脚本可能有别的许可；逐文件来源/版本仍须核对：[SQLite版权](https://www.sqlite.org/copyright.html)。TH3是单独的许可资产，排除直接复用：[TH3](https://www.sqlite.org/th3.html)。

SQLLogicTest官方Fossil about页遇浏览器验证，未取到corpus逐文件许可，状态UNAVAILABLE，不能凭SQLite总版权页给它整体授权：[官方入口](https://www.sqlite.org/sqllogictest/doc/trunk/about.wiki)。本仓已有sqllogictest crate=0.29，锁定版本见Cargo.lock；crate许可与corpus许可须分别核对，本轮不安装/复制未核corpus。C01-15先使用自写最小共同SQL fixture，实际第三方导入需版本、文件hash、许可证/保留声明清单及Reviewer确认。

proptest/Loom/SQLancer后续按阶段引用官方工具，未在本轮引入依赖；测试数量不是许可证明或正确性验收。
