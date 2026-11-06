待裁定问题（只依据材料判定，材料不足输出 INCOMPLETE）

**主张 D**：三处**既有门禁脚本**的改动都**没有放宽任何历史判据**、**没有新增任何证据红**，
且每处都有**可证伪的反向变异**证据。

改动一 check_audit_manifest.sh：
  改前漏什么：门禁用 `python3` 调 audit_run.py（模块级 import yaml）。本机 python3
    是 3.15、无 PyYAML ⇒ 每次 verify 抛 ModuleNotFoundError ⇒ 门禁把**崩溃**
    当成「证据不可复核」⇒ 21 个干净的已提交 run 全变红（红数 28 而非历史的 7），
    且红数随 shell 的 PATH 变。
  改后多报什么：不新增红；解释器不可用时只报 1 行「无法判定」并 exit 1（仍 fail closed）。
  反向变异四臂：A 默认 python3 ⇒ 7 条历史红（改前 28）；
    B 强制无 PyYAML 解释器 ⇒ 只有 1 行「cannot judge」，rc=1，零条伪造证据红；
    C 强制有 PyYAML 解释器 ⇒ 仍是那 7 条；D --selftest 仍 SELFTEST-OK。
  那 7 条历史红（改前改后逐字相同、仍判红）：1 条 run dir without manifest
    + 6 条 verify 失败，均为 2026-10 提交的审计记录（Owner 裁定项 IL-5）。

改动二 check_a04_evidence_reproducible.sh：
  改前漏什么：比对前没先清空 target/a04-5226/ ⇒ 某引擎这次不可用时不写文件，
    但上次运行的旧文件还在 ⇒ 「这次没产出」被误判成「产出了且一致」。
  改后多报什么：不新增红，只把比对域收窄为本次产出。
  反向变异：陈旧二进制下生成器的 sqlrustgo_cli 产出文件数 = 0（另三个引擎各 20）
    ⇒ 门禁报 NOT RUN 40 条、MISMATCH 0 条。

改动三 check_correctness_tests_production_storage.sh：
  改前漏什么：R1–R5 只判服务端存储装配块，测试侧无任何执行点。
  改后多报什么：不新增证据红；新增 R6（清单里每个声明了「验证存储语义」的测试
    必须被生产存储链覆盖），实现交给 audit/v4.1.0/storage_semantics_claims.py。
  反向变异：--selftest 6 臂全过，含新臂「造一份『清单里有、树上没有』的陈旧清单
    ⇒ R6 变红」。

另一处非门禁改动 expert_ladder.py 的 run_calibration：
  改前漏什么：把「没能答上来」与「答错了」压成同一个 {ok:false, answer:""}。
  改后多报什么：多出 failure ∈ {None, unavailable, wrong-answer} 与两个计数字段，
    **不改变 score/ratio 判定**。
  反向变异四臂：超时 ⇒ unavailable=4/wrong_answer=0；答错 ⇒ wrong_answer=4/unavailable=0；
    部分对 ⇒ score=1/wrong_answer=3；把源码改回旧写法 ⇒ 同一超时场景与真实现不同（检出）。

实测：check_audit_manifest.sh 7 条历史红（与改前逐字相同）；生产存储门禁 6 项判据全成立；
归档 run 20261105-gater6-001 十条命令全部 exit 0。

请回答：
1. 主张 D 是否被材料支持？
2. 三处改动里有没有哪一处**看起来**放宽了判据，或缺少可证伪的反向变异？
3. 材料若不足以支撑某处，请指出缺什么。
