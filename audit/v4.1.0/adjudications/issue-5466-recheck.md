待裁定问题（只依据材料判定，材料不足输出 INCOMPLETE）

**主张 A**：A04 已归档证据**没有漂移**（当前构建下 80/80 可比文件逐字节一致）。
**主张 B**：seats-002 掉到 3/4 的是第 4 题 code-truth-len，**根因是超时**（不是答错）。
**主张 C**：因此 #5466（p0，「已入库证据内部自相矛盾且无法从 clean checkout 复现」）核心论断不成立。

材料：

【A 的证据】构建当前 CLI 后门禁 check_a04_evidence_reproducible.sh 实测：
  PASS: 80/80 comparable archived evidence file(s) match this run byte-for-byte.
  （另 20 个 postgres 参考文件按 NOT RUN 披露 —— 参考容器不可用）
争议个案 agg_group_001（#5466 正文点名的那一条）：
  归档 docs/releases/v4.1.0/evidence/a04-5226/wire/agg_group_001_sqlrustgo_cli.txt：
    g,__agg_1,__agg_2,__agg_3,__agg_4
    x,2,3,1,2
    y,1,10,10,10
  本次同一调用方式（sqlrustgo sqlite --batch --mode csv --headers true --cmd "<setup+query>" <db>）产出：
    g,__agg_1,__agg_2,__agg_3,__agg_4
    x,2,3,1,2
    y,1,10,10,10
  ⇒ 两者一致。#5466 正文声称的归档内容（col_1..col_5 / x 1 2 2 2）在归档文件里**不存在**。

【B 的证据】seats-001 的 flash-next-252：calibration score 4/4，
  第 4 题 {"id":"code-truth-len","ok":true,"answer":"5"}，该席位 latency_s=15.0。
seats-002 的同一席位：calibration score 3/4，
  第 4 题 {"id":"code-truth-len","ok":false,"answer":""}（**空串**），latency_s=101.14。
  前 3 题两次逐字相同。
同批另一个席位 local-qwen35-9b：ok=false, latency_s=120.01,
  error="TimeoutError: timed out"。
判据（expert_ladder.py）：expect = 包含 '5' 且不含 '4'/'6'/'10' ⇒ **空串必判红**。
**注意**：seats-002 的归档记录**没有**该次调用是否超时的直接字段。
（反向变异：强制用 2026-02-21 的陈旧全局 sqlrustgo 时，门禁把 20 条判为 NOT RUN、
  MISMATCH 0 条 ⇒ 「引擎不可用」已与「证据漂移」分开记账。）

请回答：
1. A、B、C 各自是否被材料支持？（REAL=材料支持；FALSE_POSITIVE=材料反驳；INCOMPLETE=不足以判断）
2. **B 的「根因是超时」这一步**：材料只记录了「空串」，没有记录那次调用是否超时。
   同批延迟 15.0s→101.14s、同批另一席位 120s 超时属**旁证**。
   材料能否支撑 B？若只能支撑「该题返回了空串」，请如实指出。
3. C 是否被材料支持？#5466 正文的两个论断分别是「已入库证据内部自相矛盾」与
   「无法从 clean checkout 复现」—— 请分别判断。
