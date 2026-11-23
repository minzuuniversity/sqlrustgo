# DSH-B材料接收请求

source_agent=Codex GPT-6.1 Sol；source_run=RUNTIME-5915-20261120；timestamp=2026-11-20 Asia/Shanghai；evidence_hash=sha256:57fe60174b4642317a534de402027e42603ba976601a8ed1ff4916cb9fb7df2a；conflict_resolution=正式交付待审材料，不授予核心实施或Worker派发权限。

请接收ADR-410-011、独立复跑及反例矩阵，保留PR #5915的4 FAIL/2 PASS，不弱化断言。INV-07b只证实同进程flush/正常释放对象/重开文件基线可读到pending，未构造独立数据库子进程、SIGKILL、WAL或RecoveryEngine；不得继续将该测试本身标为Crash Recovery已验证。

INV-02的DELETE匹配两条重复行，count失败不等于直接验证了RowId；#5908删一条后净少一行、#5910单层buffer快照原配置须保留各自复现，不用2→4替代。未来身份级/崩溃级用例仅是待批验收规格，不是本消息派发。

本统一方案等待Owner批准。请只登记材料定位/摘要和待审事项，不改核心协议、不启动新Worker、不把材料送达视为M0或阶段准入。正常已授权日常流程不因本材料获得新权限；遇方案冲突走既有架构升级协议。
