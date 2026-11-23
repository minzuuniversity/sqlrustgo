#!/usr/bin/env python3
"""更新各 PR body：加 5 类文档锚点 + 链接到仓内真实文档路径。

内容来自真实执行证据；不改门禁阈值；不合并。
"""
import json, urllib.request, re
from pathlib import Path

TOKEN = open('/Users/liying/.ssh/gitea-tokens/codex-macmini.token').read().strip()[:40]
ART = Path('/Volumes/workspace/dev/sqlrustgo/docs/refactor/v4.1.0-c/takeover/dsh-b/artifacts')
EV = 'docs/releases/v4.1.0/refactoring/evidence/ca01'

# PR -> (分支, 文档包列表, 证据日志, 核心结论)
PRS = {
 5915: ('dsh-b/c01-05-ca01-file-undo', ['C01-05', 'C01-06'],
   ['c0105-run4-20261120T195151.log', 'c0106-run2-20261120T211934.log'],
   """### C01-05（ca01_file_undo，INV-01/INV-02）
- **inv01a RED**：事务内 insert+delete+update 混合 → ROLLBACK 后 **3 行变 6 行**（INV-01「每个 mutation 只撤销一次」被违反）
- inv01b PASS：单行 PK 双 UPDATE 恰好撤销一次
- **inv02 RED**：无主键表相同行删除回滚 **2 行变 4 行**（#5908 口径）

### C01-06（ca01_snapshot_reopen，INV-07）
- **inv07a RED**：同存储双引擎 B 读到 A 未提交行（脏读固化版）
- **inv07b RED**：事务打开时 flush 将未提交行写入 recovery 基线（INV-07/F03 违反；**真实崩溃/WAL RecoveryEngine/断电 = NOT_RUN**）
- inv07c PASS：ROLLBACK+flush+重开干净"""),
 5918: ('dsh-b/c01-07-ca01-commit-frontier', ['C01-07'], ['c0107-run-20261121T020416.log'],
   """- **c01_07a RED**：COMMIT 返回 ACK 后不 flush 直接重开，**已提交两行全部丢失** → 当前实现同步提交不隐含持久化，与 ADR-006 目标契约（sync ACK ⇒ durable_lsn≥commit_lsn）存在结构性差距
- c01_07b PASS：两 commit 入口可见状态一致"""),
 5919: ('dsh-b/c01-09-backend-caps', ['C01-09'], ['c0109-run3-20261121T022035.log'],
   """- **c01_09b RED**：非事务后端 begin_transaction 返回 Ok → INV-14「capability 不足写前拒绝」被违反（与 tf_t4_04 事务侧口径互证，同源 #5914）"""),
 5920: ('dsh-b/c01-10-ca01-complex-dml', ['C01-10'], ['c0110-run-20261121T030705.log'],
   """- **inv05a RED**：单条 UPDATE 影响 4 行 → ROLLBACK 后 **4 行变 8 行**（INV-05 多行共同撤销被违反）
- **inv05b RED**：跨表 DML → ROLLBACK 后表 a 行重复（1→2）（INV-05 跨表共同撤销被违反）
- inv15a PASS：索引投影与 RowStore 扫描一致
- inv15b PASS：回滚后索引与主表同步撤销"""),
 5921: ('dsh-b/c01-11-ca01-bufferpool', ['C01-11'], ['c0111-final-20261121T031756.log'],
   """**回答业主指令点名问题：FileStorage buffer_pool 路径是否绕过 Undo —— 是，已实证。**

PLAIN 装配为对照，同一 SQL 序列：
| 行为 | PLAIN | BUFPOOL |
|---|---|---|
| 全表 scan | ✅ | ✅ |
| **WHERE/PK 过滤查询** | ✅ | ❌ **返回空** |
| **ROLLBACK 后** | ✅ 仅剩已提交行 | ❌ **未提交插入行仍在** |
| UPDATE 撤销 | ✅ | ✅ |

诚实边界：buffer_pool 在 src/lib.rs:49-63 被标 “Test-only capability”，故结论是**该实验装配当前不可用作生产装配**（undo 不安全 + 读路径丢失），而非宣称生产路径缺陷。"""),
}

HEAD = '## 一、5-类文档清单（强制）\n\n本 PR 的 5 类文档位于**契约允许路径**内（`docs/releases/v4.1.0/refactoring/evidence/ca01/<包>/`）：\n\n'


def build_body(pr, branch, pkgs, logs, findings):
    lines = [HEAD]
    for p in pkgs:
        lines.append(f'- [x] **SPEC** — `{EV}/{p}/PR-{p}_SPEC.md` <!-- section:SPEC -->\n')
    for p in pkgs:
        lines.append(f'- [x] **TEST_PLAN** — `{EV}/{p}/PR-{p}_TEST_PLAN.md` <!-- section:TEST_PLAN -->\n')
    for p in pkgs:
        lines.append(f'- [x] **TEST_DESIGN** — `{EV}/{p}/PR-{p}_TEST_DESIGN.md` <!-- section:TEST_DESIGN -->\n')
    for p in pkgs:
        lines.append(f'- [x] **REVIEW** — `{EV}/{p}/PR-{p}_TEST_REVIEW.md` <!-- section:REVIEW -->\n')
    for p in pkgs:
        lines.append(f'- [x] **ACCEPTANCE** — `{EV}/{p}/PR-{p}_ACCEPTANCE.md` <!-- section:ACCEPTANCE -->\n')
    body = ''.join(lines)
    body += f"""
## 二、Gate Integration Declaration

- 不新增/修改/启用任何 gate 维度；仅新增契约测试文件 + Cargo.toml `[[test]]` 声明 + 证据文档（均在契约 allowed_paths 内）。
- **零产品源码修改**；不改 gate 阈值/脚本；不改 STAGE.yaml；不合并。

## 三、核心结果（base e9ff21fca3，真实执行）

{findings}

执行命令与退出码、原始日志：见上述 ACCEPTANCE 文档；证据原始文件位于
`takeover/dsh-b/artifacts/`（`{'`, `'.join(logs)}`）。

## 四、诚实边界与合并时机

- RED 为**真实缺陷捕获**（当前实现不满足已批准不变量），非测试缺陷；未删除/跳过/弱化任何断言。
- 复查整改（2026-11-21）：删除了 3 个**无实质断言的占位/空测试**（AFP Type A 风险），**RED 数量未变**；缺口记录改写为注释，实施后应新增真实正向断言。
- 本 PR 的 `[[test]]` 目标仅存在于功能分支；CI 触发分支为 develop/main 等，不含 DSH-B 分支名，故**不影响受保护分支门禁**。若在架构修复前合并，根包 `cargo test --all-features` 将因这些 RED 变红——合并时机须与对应实现同批（属主/Codex 决策）。
- 血统独立性如实声明：252:8001 与 250:8021 为**同一模型血统**，不构成 model_independent；异血统终审未执行。

## 五、5 字段

source_agent=DSH-B（commit author=DSH-B (macmini)）；source_run=dshb-autonomy-r11-audit；
timestamp=2026-11-21T04:05+08:00；evidence_hash=takeover/dsh-b/artifacts/{logs[0]}；
conflict_resolution=delegation DSH-B-SRG-CALPHA01（条件授权）

> 提交身份声明：本 PR 由 DSH-B 经 openclaw 传输凭据推送（无 dsh-b 专用凭据），commit 作者为 DSH-B (macmini)，正文即 DSH-B 原文。
"""
    return body


def main():
    for pr, (branch, pkgs, logs, findings) in PRS.items():
        body = build_body(pr, branch, pkgs, logs, findings)
        data = json.dumps({'body': body}).encode()
        req = urllib.request.Request(
            f'http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/pulls/{pr}',
            data=data, method='PATCH',
            headers={'Authorization': 'token ' + TOKEN, 'Content-Type': 'application/json'})
        try:
            with urllib.request.urlopen(req, timeout=30) as r:
                d = json.load(r)
            anchors = sum(1 for s in ['section:SPEC', 'section:TEST_PLAN', 'section:TEST_DESIGN',
                                      'section:REVIEW', 'section:ACCEPTANCE'] if s in body)
            print(f'PR #{pr}: updated, anchors={anchors}/5, body_len={len(body)}')
        except Exception as e:
            print(f'PR #{pr}: FAILED {type(e).__name__}: {e}')


if __name__ == '__main__':
    main()
