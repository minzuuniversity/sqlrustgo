# AOCI-CODE 索引状态独立核验（2026-11-08）

> **本文件的性质**：对已合入 `develop/v4.1.0` 的 AOCI-CODE 认知层的**独立核验记录**，
> 不是构建报告。构建过程见 §2 引用的提交。
> 核验环境：aoci `0.1.0-rc20`（`/home/openclaw/.local/bin/aoci`），
> 干净 clone `http://192.168.0.252:3000/openclaw/sqlrustgo.git` @ `69db236`。

## 1. 结论摘要

| 项 | 结论 |
|---|---|
| 索引是否合入 develop | ✅ 是（`docs/iaia-410-aoci-index` 已并入） |
| 条目总数 | **5285** |
| 结构有效性 | ✅ `structure_valid: true` |
| **治理对齐** | ❌ **未对齐**（`governance_aligned: false`，`next_action: blocked`） |
| `aoci check` 退出码 | **1**（非 0） |
| findings | **1393** |

**索引已建完，但当前不处于「aligned」状态。** 原因不是条目写错，而是
**代码在索引建立后继续大幅变动**，索引落后于 HEAD。

## 2. 核验方法

```bash
git clone --depth 1 --branch develop/v4.1.0 \
  http://192.168.0.252:3000/openclaw/sqlrustgo.git aoci_verify
cd aoci_verify
aoci check --repo . --json > check.json
```

`check` 输出（`rc20`）：

```json
{"ok": false, "exit_code": 1,
 "structure_valid": true, "governance_aligned": false,
 "next_action": "blocked", "findings": [...1393 items...]}
```

## 3. findings 分类

| code | 数量 | 含义 |
|---|---:|---|
| `code_missing` | 461 | 文件在 Managed Scope 内但无条目 |
| `code_unbaselined` | 460 | 文件未进入 baseline 指纹 |
| `code_stale` | 313 | 条目存在但源文件指纹已变（文件被改过） |
| `code_skipped` | 146 | 按规则跳过（binary/empty/oversize），非缺陷 |
| `code_orphan` | 12 | 条目存在但文件已不在仓库 |
| `observed_pending` | 1 | observe 角色待人工确认 |

**这 1393 项里没有一条是「条目内容错误」** —— 全部是**代码变动导致的漂移**：

- `code_stale` 的样例：`.gitea/workflows/ci.yml`、`.gitea/workflows/gate.yml`
  —— 这些文件在索引建立后被改过（CI 管道修复 #5187、门禁加固 #5184 等）。
- `code_missing` / `code_unbaselined` 的样例：`audit/v4.1.0/**`、
  `.gitea/workflows/ci-trusted-baseline.yml` —— 审计整改过程中新增的文件。

**结论：索引需要一次增量刷新（Maintain），而不是重写。**

## 4. 条目质量抽样核验

对 `develop/v4.1.0:aoci.code.txt` 全量 5285 条做 F 字段前缀分类：

| 类别 | 数量 | 占比 |
|---|---:|---:|
| 描述性（`Documents…` / `Runs…` / `Declares module…` / `Provides…`） | 3516 | **67%** |
| 机械抽取（`Starts with…` / `Declares top-level key…`） | 541 | **10%** |
| 其他（描述长度不足或格式特殊） | 1228 | **23%** |

**机械抽取的样例**（这些是弱条目，应人工重写）：

```
.gitignore[DB4M]: F:Starts with gate_report.json | ...
CODEOWNERS[DB4T]: F:Starts with * @yinglichina8848 | ...
```

**相对好的条目**（抽取时确实读到了语义）：

```
.gitattributes[EG4T]: F:Routes Git LFS filters for the bulk data and archive
  extensions required by the gitcode.com pre-receive hook | ...
ARCHITECTURE_RULES.md[SB4M]: F:Documents SQLRustGo AI 协作安全规则 as
  documentation | ...
```

### 4.1 与 IAIA-410 §4.2 证据链的关系

§4.2 要求「功能声明 → 真实实现 → 运行时调用 → 外部可观测」四类证据缺一不可。
AOCI 条目只覆盖第一类（**声明**），且其中 10% 是机械抽取的弱声明。

**因此 AOCI 索引不构成 §4.2 的证据链**，它是一个导航层：
告诉 Agent「这个文件声称负责什么」，不能告诉 Agent「它是否真的做到了」。

这与 §15.4 的既有记录一致，且不改变 §15.3 的 GA 判定。

## 5. 索引规模与认知预算

| 项 | 值 |
|---|---|
| baseline 文件 | 6389（index 5435 / observe 954） |
| 条目数 | 5285 |
| `whole_index.target_tokens` | 200000 |
| `whole_index.max_tokens` | 600000 |
| `cognition_budget.mode` | enforce |

预算口径为 `target 200K / max 600K`。5285 条目的实际 token 量需以
`aoci ui` 或 `aoci index agent guide` 复核；本次未测量，故不给出估算值。

## 6. 与 §15 整改项的关系

§15 的 R1–R10 整改项中，**AOCI 索引不关闭其中任何一项**：

- R1（CLUSTERED 绕过 WAL，GA 阻断）—— 需 WAL 层整改，与索引无关；
- R2/R3（变异 spec 缺失、11 条 REVIEW 定性）—— 需测试整改；
- R4（差分测试）—— 需外部 DBMS 对照；
- R8（跨版本回归）、R9（独立审核）—— 需执行。

索引的**唯一实际作用**是降低后续整改中 Agent 重复读码的成本，
并给 R4/R8 的差分测试提供「从哪里开始看」的导航。

## 7. 建议的下一步（按投入产出）

1. **增量刷新索引**（`aoci maintain` 多轮）——
   313 `code_stale` + 461 `code_missing` 可由增量维护覆盖，
   使 `aoci check` 回到 exit 0 / `governance_aligned: true`。
   这是**唯一**能立即改善门禁状态的项。
2. **重写 541 条机械条目** —— 这些是「F:Starts with …」形态，
   信息量近零；重写为语义描述后索引才有导航价值。
3. **不把索引当作验证证据** —— §15.3 的 GA 阻断项与索引质量无关。

## 8. 复现命令

```bash
git clone --depth 1 --branch develop/v4.1.0 \
  http://192.168.0.252:3000/openclaw/sqlrustgo.git aoci_verify
cd aoci_verify
aoci check --repo . --json | python3 -c "
import json,sys,collections
d=json.load(sys.stdin)
print('exit',d['exit_code'],'structure_valid',d['structure_valid'],
      'governance_aligned',d['governance_aligned'])
c=collections.Counter(x['code'] for x in d['findings'])
print(dict(c))"
```

## 9. 关联

- `docs/releases/v4.1.0/IAIA_410_AUDIT_PLAN.md` §4.2 / §13 / §15
- `docs/agent-harness/AOCI_GUIDE.md`
- Issue #5196（AOCI-CODE 评估：可作导航层，不能替代 IAIA-410）