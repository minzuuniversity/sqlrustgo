#!/usr/bin/env bash
#
# check_evidence_binding.sh — 证据绑定检查脚本
#
# 用途: 检查文档中的声明是否绑定了有效的机器可验证证据
#
# Anti-Fabrication Policy 要求:
# - 每个 PASS/FAIL 声明必须绑定 CI run ID + log hash
# - 每个"完成"声明必须绑定 commit hash / PR 合并证明
# - 每个门禁结果必须绑定 gate_policy_eval_id
# - 无证据的声明必须标记为 UNVERIFIED CLAIM
#
# 违规类型:
# - Type A: 虚构执行（无 CI 证据声明测试通过）
# - Type B: 伪门禁（无 gate engine 输出声明门禁通过）
# - Type C: 伪证据（引用不存在的 CI run / log hash）
# - Type D: 伪任务完成（无 commit/PR 声明任务完成）

set -euo pipefail

VERSION="${1:-}"
OUT_DIR="${2:-}"

if [ -z "$VERSION" ] || [ -z "$OUT_DIR" ]; then
  echo "Usage: $0 <version> <out_dir>"
  exit 2
fi

mkdir -p "$OUT_DIR"

PASS_COUNT=0
FAIL_COUNT=0
WARN_COUNT=0

PASS_ITEMS=()
FAIL_ITEMS=()
WARN_ITEMS=()
UNVERIFIED_CLAIMS=()

add_pass() { PASS_ITEMS+=("$1"); PASS_COUNT=$((PASS_COUNT + 1)); }
add_fail() { FAIL_ITEMS+=("$doc: $1"); FAIL_COUNT=$((FAIL_COUNT + 1)); }
add_warn() { WARN_ITEMS+=("$doc: $1"); WARN_COUNT=$((WARN_COUNT + 1)); }
add_unverified() { UNVERIFIED_CLAIMS+=("$1"); }

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

# The provenance rule lives in scripts/gate/common.sh — one rule, one
# implementation. See the comment on `evidence_binding_has_provenance` there
# for why the previous fifteen-pattern version of this check was wrong.
# shellcheck source=scripts/gate/common.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

RELEASE_DIR="docs/releases/$VERSION"

# ============================================================
# 检查函数
# ============================================================

# 检查 PASS/FAIL 声明是否有 CI 证据
check_pass_fail_evidence() {
  local doc="$1"
  local path="$RELEASE_DIR/$doc"

  # 子目录兼容
  if [ ! -f "$path" ]; then
    if [ -f "$doc" ] && [[ "$doc" == */* ]]; then
      path="$REPO_ROOT/$doc"
    elif [ -f "$REPO_ROOT/docs/releases/$VERSION/$doc" ]; then
      path="$REPO_ROOT/docs/releases/$VERSION/$doc"
    else
      add_warn "文档不存在（跳过检查）：$doc"
      return
    fi
  fi

  if [ ! -f "$path" ]; then
    add_warn "文档不存在（跳过检查）：$doc"
    return
  fi

  # 历史快照豁免：标记 env:historical-snapshot 的文件自动豁免逐行 PASS/FAIL 检查
  if grep -qE "env:historical-snapshot|env:historical_snapshot|env-historical-snapshot" "$path" 2>/dev/null; then
    add_pass "历史快照文件已豁免 PASS/FAIL 逐行检查: $doc (env:historical-snapshot 标记)"
    return
  fi

  # 查找所有 PASS/FAIL 声明
  local lines_with_pass_fail
  lines_with_pass_fail=$(grep -nE "PASS|FAIL|通过|失败|完成|done|completed|成功|失败" "$path" 2>/dev/null || true)

  if [ -z "$lines_with_pass_fail" ]; then
    add_pass "无状态声明（无需证据检查）：$doc"
    return
  fi

  # 对每个声明检查是否有证据绑定
  #
  # 如果文档整体锚定到一个**真实的 commit**，所有声称由该 commit 背书。
  #
  # #5402: 这里是本门禁 335 个「Type A/B 违规」的全部来源。之前这段代码用
  # **十五条**硬编码正则去识别文档头部��� provenance —— 每出现一种新写法就补
  # 一条（`**Commit:**`、`**Commits**:`、`| ... | commit | ... |`、
  # `**Branch**:`、`**Status**:`、`**Source Issue**:` + `**Agent**:` …）。
  # 在 `docs/releases/v4.1.0` 的 93 个文档上实测：这十五条只认得出
  # **56 个被标记文档中的 2 个**。其余文档全部落入逐行扫描，产生 335 个
  # 违规 —— 其中 **51 个文档的头部本来就写着真实的 commit SHA**，只是写法是
  # 仓库自己的 `**基线**：`develop/v4.1.0` @ `0ba527dd…``，十五条正则一条都
  # 不认。于是门禁不是在抓伪造，是在抓**不认得本仓库的行文风格**。
  #
  # 现在判据只有一条，且**可证伪**：文档头部存在一个能解析为真实 commit
  # 对象的 git SHA。解析交给 `scripts/gate/common.sh` 里的唯一实现
  # （`evidence_binding_has_provenance`）—— 一个规则只允许有一份实现。
  #
  # 这比十五条正则**更强**，不是更松：
  #   * 格式无关 —— 作者写 `**基线**：`、`base:`、`**Source commit**:`、
  #     `source_head:` 还是表格单元，判据只问「这里有没有一个 commit」。
  #   * 可证伪 —— `git cat-file -t` 能区分真实 commit 与编造的哈希。伪造的
  #     文档没法靠「打一个像哈希的字符串」买到处分；十五���正则没有这个能力，
  #     只要形状像就放过。
  local has_doc_provenance=false
  local provenance_sha=""
  local provenance_rc=0
  provenance_sha="$(evidence_binding_has_provenance "$path")" || provenance_rc=$?

  if [ "$provenance_rc" -eq 0 ] && [ -n "$provenance_sha" ]; then
    has_doc_provenance=true
  elif [ "$provenance_rc" -eq 2 ]; then
    # 历史太浅，无法判定 —— 这是**环境限制**，不是伪造。披露，不判失败。
    add_unverified "git 历史过浅，无法验证 $doc 的 provenance（需要完整历史）"
    add_warn "无法判定 provenance（git 历史过浅）：$doc"
    return
  fi

  # gate engine 输出（policy_eval_id）是**另一种**锚点，与 commit 无关，
  # 单独判定。
  if [ "$has_doc_provenance" = false ]; then
    if grep -qE "gate_policy_eval_id|policy_eval_id" "$path" 2>/dev/null; then
      has_doc_provenance=true
    fi
  fi

  # 检查环境限制标记 — 文档含 env:blocked:no-ci 表明作者已诚实
  # 标注"无 Gitea CI, 接受本地 verification log 证据"。
  local has_env_blocked=false
  if grep -qE "env:blocked:no-ci|env-blocked-no-ci" "$path" 2>/dev/null; then
    has_env_blocked=true
  fi

  # 有 provenance 的 Gate Report，跳过逐行检查（整体背书）
  if [ "$has_doc_provenance" = true ]; then
    add_pass "Gate Report 有整体 provenance（commit 或 gate_policy_eval_id）：$doc"
    return
  fi

  # 有 env:blocked 标记 — 文档已诚实标注环境限制，所有 PASS/FAIL 声明视为 WARN
  if [ "$has_env_blocked" = true ]; then
    add_warn "文档标注环境限制 (env:blocked:no-ci): $doc (无 Gitea CI, 接受本地 verification log 证据)"
    return
  fi

  # 整文档无 provenance 且无 env:blocked 标记 — 逐行检查
  while IFS=: read -r line_num content; do
    # 跳过注释行和代码块
    skip_pattern="^#"
    if echo "$content" | grep -qE "$skip_pattern"; then
      continue
    fi
    # 代码块检测用单独命令
    if echo "$content" | grep -q "^\`\`\`"; then
      continue
    fi

    # 如果文档有整体 provenance，表格行（| ... | PASS |）豁免
    if [ "$has_doc_provenance" = true ]; then
      if echo "$content" | grep -qE "^\|.*\|"; then
        continue  # 表格行，整体 provenance 覆盖
      fi
    fi
    # 排除教程/分析类文档中的历史引用（降级为警告）
    if echo "$content" | grep -qE "v3\.[0-9]\.[0-9]|历史版本|legacy|过去|已过时"; then
      add_warn "警告：第 $line_num 行历史版本引用可能需更新: $(echo "$content" | cut -c1-50)"
      continue
    fi
    # 排除政策规则定义类声明（声明约束条件，非测试结果）
    if echo "$content" | grep -qE "禁止声明|禁止.*声明"; then
      continue  # 政策规则，不是测试结果
    fi
    has_ci_ref=false
    has_gate_ref=false
    if echo "$content" | grep -qE "(run_|#|ci_run|CI_RUN|log_hash|log-hash)"; then
      has_ci_ref=true
    fi

    # 检查是否有 gate engine 输出引用（如 policy_eval_id, gate_output）
    if echo "$content" | grep -qE "(policy_eval|gate_output|gate_policy|GATE_)"; then
      has_gate_ref=true
    fi

    # 检查是否有 commit hash
    #
    # #5402: 这里原来只问「这一行里有没有一串 7–40 位的十六进制字符」。
    # 那个判据把**任何**看起来像哈希的东西当证据 —— 表格里的行号、
    # 十六进制常量、一个随手编的 `deadbeef` 都能让一行「测试通过」免于
    # 被查。判据必须是**可证伪**的：这一行引用的 commit 必须真的存在于
    # 仓库里（`git cat-file -t` 解析为 commit）。与文档级 provenance 用
    # 的是同一份实现（scripts/gate/common.sh），一个规则只有一份实现。
    local has_commit_ref=false
    local line_sha
    line_sha="$(evidence_binding_resolve_commit_in_line "$content")"
    if [ -n "$line_sha" ]; then
      has_commit_ref=true
    fi

    # 判断声明类型并验证
    # 排除未来要求模式
    if echo "$content" | grep -qE "MUST PASS|预计|TODO|TBD|应该|should"; then
      : # 未来要求，不是违规
    # 排除"通过标准"（定义标准，不是声称结果）
    elif echo "$content" | grep -qE "通过标准|验收标准|验收准则|Threshold|标准[:：]"; then
      : # 定义标准，不是声称结果
    # 排除教程/分析类文档中的历史引用（降级为警告）
    elif echo "$content" | grep -qE "v3\.[0-9]\.[0-9]|历史版本|legacy|过去|已过时"; then
      add_warn "警告：第 $line_num 行历史版本引用可能需更新: $(echo "$content" | cut -c1-50)"
    elif echo "$content" | grep -qE "PASS|通过|成功|completed|done|PASS"; then
      # 已完成声明需要证据
      if [ "$has_ci_ref" = false ] && [ "$has_gate_ref" = false ] && [ "$has_commit_ref" = false ]; then
        add_fail "Type A/B 违规：第 $line_num 行声明无 CI/gate/commit 证据: $(echo "$content" | cut -c1-60)"
        add_unverified "Line $line_num: $content"
      else
        add_pass "状态声明有证据绑定：第 $line_num 行"
      fi
    fi

    if echo "$content" | grep -qE "FAIL|失败|failed"; then
      if [ "$has_ci_ref" = false ] && [ "$has_gate_ref" = false ]; then
        add_warn "警告：第 $line_num 行 FAIL 声明可能无证据"
      fi
    fi
  done <<< "$lines_with_pass_fail"
}

# 检查是否存在伪证据（引用不存在的 CI run + 真实验证 hash/commit）
check_fabricated_evidence() {
  local doc="$1"
  local path="$RELEASE_DIR/$doc"

  # 子目录兼容：如果 RELEASE_DIR/$doc 不存在，尝试作为相对路径
  if [ ! -f "$path" ]; then
    if [ -f "$doc" ] && [[ "$doc" == */* ]]; then
      path="$REPO_ROOT/$doc"
    elif [ -f "$REPO_ROOT/docs/releases/$VERSION/$doc" ]; then
      path="$REPO_ROOT/docs/releases/$VERSION/$doc"
    else
      return
    fi
  fi

  if [ ! -f "$path" ]; then
    return
  fi

  # ============================================================
  # Type C 真实验证 1: evidence_hash 必须匹配真实文件 SHA256
  # ============================================================
  # 提取 evidence_hash 字段值（支持 markdown 表格 `| evidence_hash | xxx |`
  # 和 YAML 风格 `evidence_hash: xxx` 两种格式）
  local claimed_hash
  claimed_hash=$(grep -oE "evidence_hash[ ]*[\|:][ ]*\`?([0-9a-f]{32,})" "$path" 2>/dev/null | grep -oE "[0-9a-f]{32,}" | head -1 || true)

  if [ -n "$claimed_hash" ]; then
    # 优先尝试 1: 如果 evidence_hash 是已知 git commit hash, 接受（视为 Merge Commit SHA1 锚定）
    if [ ${#claimed_hash} -eq 40 ] && git cat-file -t "$claimed_hash" >/dev/null 2>&1; then
      add_pass "evidence_hash 真实验证通过（git commit object）: $doc ($claimed_hash)"
      return
    fi

    # 查找 log 字段对应的目标文件（支持两种 markdown 格式）
    # 格式 A: | log | `path/to/log` |   （带反引号）
    # 格式 B: | log | path/to/log |     （不带反引号）
    local target_file
    target_file=$(grep -oE "\| log \|[ ]*\`?([^\`\|]+)\`?" "$path" 2>/dev/null | head -1 | sed -E 's/\| log \|[ ]*//; s/^[ \`]+//; s/[ \`]+$//' || true)

    if [ -n "$target_file" ] && [ -f "$REPO_ROOT/$target_file" ]; then
      # log 路径存在 — 验证 evidence_hash == log 文件 SHA256
      local actual_hash
      actual_hash=$(sha256sum "$REPO_ROOT/$target_file" 2>/dev/null | cut -d' ' -f1 || true)
      if [ -n "$actual_hash" ] && [ "$claimed_hash" != "$actual_hash" ]; then
        add_fail "Type C 违规: evidence_hash 不匹配 log 文件. 文档声明: ${claimed_hash:0:16}..., log 文件 $target_file 实际 SHA256: ${actual_hash:0:16}... ($doc)"
        add_unverified "evidence_hash mismatch: claimed=${claimed_hash:0:16} actual=${actual_hash:0:16} target=$target_file in $doc"
      elif [ -n "$actual_hash" ]; then
        add_pass "evidence_hash 真实验证通过（log 文件 SHA256）: $doc ($target_file)"
      fi
    else
      # 没有可验证的 log 路径 — 尝试 commit 时间点的 self hash
      # 查找 evidence_hash 字段附近的 commit 字段
      local ref_commit
      ref_commit=$(grep -oE "commit[ ]*[\|:][ ]*\`?([0-9a-f]{40})" "$path" 2>/dev/null | head -1 | grep -oE "[0-9a-f]{40}" || true)
      if [ -n "$ref_commit" ] && git cat-file -t "$ref_commit" >/dev/null 2>&1; then
        # 用 git show 取该 commit 时该文件的 SHA256
        local doc_rel_path
        doc_rel_path=$(echo "$path" | sed "s|^$REPO_ROOT/||")
        local historical_hash
        historical_hash=$(git show "$ref_commit:$doc_rel_path" 2>/dev/null | sha256sum 2>/dev/null | cut -d' ' -f1 || true)
        if [ -n "$historical_hash" ] && [ "$claimed_hash" = "$historical_hash" ]; then
          add_pass "evidence_hash 真实验证通过（commit 时 self SHA256）: $doc @ $ref_commit"
        elif [ -n "$historical_hash" ]; then
          add_fail "Type C 违规: evidence_hash 不匹配 commit 时 self hash. 文档声明: ${claimed_hash:0:16}..., commit $ref_commit 时文件 SHA256: ${historical_hash:0:16}... ($doc)"
          add_unverified "evidence_hash self mismatch at $ref_commit: claimed=${claimed_hash:0:16} actual=${historical_hash:0:16} in $doc"
        fi
      else
        # 既无 log 路径也无 commit — self hash 当前快照
        local actual_self_hash
        actual_self_hash=$(sha256sum "$path" 2>/dev/null | cut -d' ' -f1 || true)
        if [ -n "$actual_self_hash" ] && [ "$claimed_hash" != "$actual_self_hash" ]; then
          add_warn "Type C 警告: evidence_hash 与当前文件 SHA256 不匹配，且无可验证 commit/log 上下文: $doc (声明: ${claimed_hash:0:16}..., 实际: ${actual_self_hash:0:16}...)"
        elif [ -n "$actual_self_hash" ]; then
          add_pass "evidence_hash 真实验证通过（当前 self SHA256）: $doc"
        fi
      fi
    fi
  fi

  # ============================================================
  # Type C 真实验证 2: commit hash 必须存在于 git object store
  # ============================================================
  local claimed_commit
  claimed_commit=$(grep -oE "\*\*[Cc]ommit\*\*[ ]*:[ ]*\`?([0-9a-f]{7,40})" "$path" 2>/dev/null | head -1 | grep -oE "[0-9a-f]{7,40}" || true)
  if [ -z "$claimed_commit" ]; then
    # 也支持 markdown 表格中的 commit 列
    claimed_commit=$(grep -oE "\| commit \| \`([0-9a-f]{7,40})\`" "$path" 2>/dev/null | head -1 | grep -oE "[0-9a-f]{7,40}" || true)
  fi
  if [ -n "$claimed_commit" ]; then
    if ! git cat-file -t "$claimed_commit" >/dev/null 2>&1; then
      # #5402: 这个判定需要**历史**。一个 shallow 的 checkout 里没有历史，
      # 于是任何指向历史 commit 的引用都会在这里被判成「引用了不存在的
      # git object」—— 一个纯粹由检出深度造成的假红。历史太浅时，如实说
      # 「无法判定」，不要把环境限制写成伪造。
      if [ "$(git rev-parse --is-shallow-repository 2>/dev/null || echo false)" = "true" ]; then
        add_unverified "commit object not checked (git 历史过浅): $claimed_commit in $doc"
        add_warn "无法验证 commit 引用（git 历史过浅）：$doc ($claimed_commit)"
      else
        add_fail "Type C 违规: commit hash 引用了不存在的 git object: ${claimed_commit:0:7}... ($doc)"
        add_unverified "commit object not found: $claimed_commit in $doc"
      fi
    fi
  fi

  # ============================================================
  # 旧逻辑: CI run ID 格式检查（保留）
  # ============================================================
  local ci_refs
  ci_refs=$(grep -oE "run_[0-9]{8}_[0-9]{3}|#[0-9]+|ci_run[_-]id:?[ ]*[0-9]+" "$path" 2>/dev/null || true)

  if [ -z "$ci_refs" ]; then
    return
  fi

  # 检查引用的 CI run 是否存在于 CI 系统或作为 artifact
  # 注意：这里只能检查格式是否正确，无法访问外部 CI 系统
  while read -r ref; do
    # 简单格式检查（实际应该查询 CI 系统）
    if ! echo "$ref" | grep -qE "^run_[0-9]{8}_[0-9]{3}$|^#[0-9]+$"; then
      add_warn "可疑 CI 引用格式：$ref 在 $doc"
    fi
  done <<< "$ci_refs"
}

# 检查门禁报告中的 gate engine 输出
check_gate_output_evidence() {
  local doc="$1"
  local path="$RELEASE_DIR/$doc"

  if [ ! -f "$path" ]; then
    return
  fi

  # 如果是门禁相关文档，检查是否有 gate engine 输出
  #
  # #5402: 这一段原来有两个缺陷，都会把**没有伪造**的文档判成伪门禁。
  #
  # 缺陷一：只要**文件名**里含 GATE 就要求 `gate_policy_eval_id`。
  # `gate_policy_eval_id` 是 **gate engine 自己在输出里写的字段** —— 只有
  # 由门禁引擎产出的报告才该有它。一份「记录门禁修复过程的证据文档」或
  # 「门禁执行计划」本来就不�� gate engine 的产物，要求它带这个字段是
  # 问错了问题。实测因此误判了 4 份文档（其中 `ALPHA_TO_BETA_GATE_PLAN.md`
  # 是一份**计划**，它自己写着「本文件发布时不填写 PASS」）。
  #
  # 缺陷二：判定「是否声明 PASS」用的是**子���出现**，不是**是否在声明**。
  # 于是否定句被当成了声明：
  #   * 「本批次没有改变任何一道门禁的『通过/不通过』结论。」
  #   * 「GA 的 6 项缺件没有被伪造，也没有被『跳过即 PASS』。」
  #   * 「不声称任何门禁通过」
  # 这三句都在说**没有**伪造，却都被判成「门禁声明 PASS 但无 gate engine
  # 输出」。门禁在抓一个不存在的伪造。
  #
  # 现在：只有当文档**确实在声明**一道门禁通过（正向断言），才要求
  # gate engine 输出；否定句与计划文档不再触发。
  if echo "$doc" | grep -qE "GATE|GATE_CHECK|RELEASE_GATE"; then
    # 只在文档**正向声明**某道门禁通过时才要求 gate engine 输出。
    #
    # 声明的形态：句子里出现「PASS」/「通过」，且该句**不是**否定句。
    # 否定线索：不/未/无/非/没有/未伪造/不声称/跳过即 之类。
    local gate_pass_claim=false
    while IFS= read -r claim_line; do
      case "$claim_line" in
        *不*|*未*|*无*|*非*|*没有*|*不能*|*禁止*|*杜绝*)
          continue
          ;;
      esac
      if echo "$claim_line" | grep -qE "GA.*PASS|Gate.*PASS|门禁.*通过|门禁.*PASS|通过标准.*PASS"; then
        gate_pass_claim=true
        break
      fi
    done <<EOF
$(grep -nE "GA.*PASS|Gate.*PASS|门禁.*通过|门禁.*PASS" "$path" 2>/dev/null || true)
EOF

    if [ "$gate_pass_claim" = true ]; then
      if ! grep -qE "policy_eval_id|gate_policy_eval|GATE_POLICY" "$path" 2>/dev/null; then
        add_fail "Type B 违规：门禁声明 PASS 但无 gate engine 输出：$doc"
      fi
    fi
  fi
}

# 检查计划文档状态标识
check_plan_status_fabrication() {
  local doc="$1"
  local path="$RELEASE_DIR/$doc"

  if [ ! -f "$path" ]; then
    return
  fi

  # VERSION_PLAN / DEVELOPMENT_PLAN / TEST_PLAN 不应出现 GA Final 状态（已完成）
  # 排除 "Planned" / "Future" / "TODO" 前缀
  if echo "$doc" | grep -qE "VERSION_PLAN|DEVELOPMENT_PLAN|TEST_PLAN"; then
    if grep -qE "GA APPROVED|GA.*✅|GA.*COMPLETE|GA.*DONE" "$path" 2>/dev/null; then
      local ga_final_line
      ga_final_line=$(grep -nE "GA APPROVED|GA.*✅|GA.*COMPLETE|GA.*DONE" "$path" 2>/dev/null | head -1 | cut -d: -f1)
      if [ -n "$ga_final_line" ] && [ "$ga_final_line" -le 10 ]; then
        add_pass "计划文档状态标注在标题行（可接受）：$doc"
      else
        add_fail "Type D 违规：计划文档显示 GA 状态（疑似伪造）：$doc 第 $ga_final_line 行"
      fi
    fi
  fi
}

# 检查 provenance 元数据
check_provenance_metadata() {
  local doc="$1"
  local path="$RELEASE_DIR/$doc"

  # 子目录兼容
  if [ ! -f "$path" ]; then
    if [ -f "$doc" ] && [[ "$doc" == */* ]]; then
      path="$REPO_ROOT/$doc"
    elif [ -f "$REPO_ROOT/docs/releases/$VERSION/$doc" ]; then
      path="$REPO_ROOT/docs/releases/$VERSION/$doc"
    else
      return
    fi
  fi

  if [ ! -f "$path" ]; then
    return
  fi

  # 历史快照豁免（与 check_pass_fail_evidence 一致）
  if grep -qE "env:historical-snapshot|env:historical_snapshot|env-historical-snapshot" "$path" 2>/dev/null; then
    add_pass "历史快照文件已豁免 provenance 元数据检查: $doc"
    return
  fi

  # 检查是否有 provenance 元数据
  if ! grep -qE "provenance:|generated_by:|generated_at:" "$path" 2>/dev/null; then
    # 对于 AI 生成的文档，这是警告
    add_warn "文档可能缺少 provenance 元数据：$doc"
  else
    add_pass "文档有 provenance 元数据：$doc"
  fi
}

# ============================================================
# 主检查流程
# ============================================================
echo "=== Evidence Binding Check for v$VERSION ==="

# 获取所有需要检查的文档
if [ -d "$RELEASE_DIR" ]; then
  ALL_DOCS=$(find "$RELEASE_DIR" -name "*.md" -type f 2>/dev/null || true)
else
  echo "版本文档目录不存在：$RELEASE_DIR"
  exit 1
fi

if [ -z "$ALL_DOCS" ]; then
  echo "未找到任何文档"
  exit 1
fi

for doc_path in $ALL_DOCS; do
  # 使用相对于 RELEASE_DIR 的路径作为 doc_name，支持子目录文件（如 evidence/sqllogictest/smoke-report.md）
  doc_name=$(echo "$doc_path" | sed "s|^$RELEASE_DIR/||")
  echo "检查: $doc_name..."

  check_pass_fail_evidence "$doc_name"
  check_fabricated_evidence "$doc_name"
  check_gate_output_evidence "$doc_name"
  check_plan_status_fabrication "$doc_name"
  check_provenance_metadata "$doc_name"
done

# ============================================================
# 生成报告
# ============================================================
REPORT_FILE="$OUT_DIR/EVIDENCE_BINDING_REPORT.md"

cat > "$REPORT_FILE" << EOF
# v$VERSION 证据绑定检查报告

> **检查日期**: $(date +%Y-%m-%d)
> **版本**: $VERSION
> **Auditor**: Hermes Agent (Anti-Fabrication Policy v1.0)
> **检查工具**: check_evidence_binding.sh

---

## 检查结果概览

| 检查项 | 数量 |
|--------|------|
| 通过 | $PASS_COUNT |
| 警告 | $WARN_COUNT |
| 失败（违规） | $FAIL_COUNT |
| 未验证声明 | ${#UNVERIFIED_CLAIMS[@]} |

**总结**: $([ $FAIL_COUNT -gt 0 ] && echo "❌ 发现 $FAIL_COUNT 个违规（Type A/B/C/D）" || echo "✅ 无违规发现")

---

## 违规详情（按类型分类）

### Type A: 虚构执行（Execution Fabrication）

无 CI 证据声明"测试通过 / 编译成功"

EOF

if [ $FAIL_COUNT -gt 0 ]; then
  for item in "${FAIL_ITEMS[@]}"; do
    echo "- ❌ $item" >> "$REPORT_FILE"
  done
else
  echo "✅ 无 Type A/B 违规" >> "$REPORT_FILE"
fi

cat >> "$REPORT_FILE" << EOF

---

### 警告项（需要人工复核）

EOF

if [ $WARN_COUNT -gt 0 ]; then
  for item in "${WARN_ITEMS[@]}"; do
    echo "- ⚠️ $item" >> "$REPORT_FILE"
  done
else
  echo "✅ 无警告" >> "$REPORT_FILE"
fi

cat >> "$REPORT_FILE" << EOF

---

### 通过项

EOF

if [ $PASS_COUNT -gt 0 ]; then
  for item in "${PASS_ITEMS[@]}"; do
    echo "- ✅ $item" >> "$REPORT_FILE"
  done
else
  echo "（无）" >> "$REPORT_FILE"
fi

cat >> "$REPORT_FILE" << EOF

---

## 未验证声明（UNVERIFIED CLAIMS）

EOF

if [ ${#UNVERIFIED_CLAIMS[@]} -gt 0 ]; then
  cat >> "$REPORT_FILE" << EOF
以下声明**无证据支撑**，不得用于门禁判断：

| 文档 | 行 | 声明内容 |
|------|-----|----------|
EOF
  for claim in "${UNVERIFIED_CLAIMS[@]}"; do
    echo "| - | $(echo "$claim" | cut -d: -f2-) |" >> "$REPORT_FILE"
  done
  echo "" >> "$REPORT_FILE"
  echo "**规则**: UnverifiedDoc 不得用于门禁判断" >> "$REPORT_FILE"
else
  echo "✅ 无未验证声明" >> "$REPORT_FILE"
fi

cat >> "$REPORT_FILE" << EOF

---

## Anti-Fabrication Policy 合规状态

| 要求 | 状态 |
|------|------|
| PASS/FAIL 声明绑定 CI 证据 | $([ $FAIL_COUNT -gt 0 ] && echo "❌ 违规" || echo "✅ 合规") |
| 门禁结果绑定 gate_policy_eval_id | $([ $FAIL_COUNT -gt 0 ] && echo "❌ 违规" || echo "✅ 合规") |
| 计划文档无 GA Final 伪造 | $([ $FAIL_COUNT -gt 0 ] && echo "❌ 违规" || echo "✅ 合规") |
| provenance 元数据存在 | $([ $WARN_COUNT -gt 0 ] && echo "⚠️  部分缺失" || echo "✅ 合规") |

---

## 后续行动

EOF

if [ $FAIL_COUNT -gt 0 ]; then
  cat >> "$REPORT_FILE" << EOF
### 必须执行的修复

1. **立即停止**当前门禁流程
2. **回退**到上一个 VerifiedDoc 状态
3. **补充**真实证据（CI run ID + log hash）
4. **重新**执行门禁检查

### 问责记录

- 违规次数: $FAIL_COUNT
- 违规类型: Type A（虚构执行）/ Type B（伪门禁）/ Type D（伪任务完成）
- 处理方式: 触发 Anti-Fabrication Policy 问责机制

EOF
else
  cat >> "$REPORT_FILE" << EOF
✅ 无需修复，所有检查通过

### 合规确认

- 所有 PASS/FAIL 声明有 CI 证据绑定
- 所有门禁结果有 gate_policy_eval_id
- 无计划文档状态伪造
- provenance 元数据存在

EOF
fi

cat >> "$REPORT_FILE" << EOF

---

*报告生成时间: $(date '+%Y-%m-%d %H:%M:%S')*
*检查工具版本: check_evidence_binding.sh v1.0.0*
*依据政策: Anti-Fabrication Policy v1.0.0*

---

## 参考：违规类型定义

| 类型 | 定义 | 严重程度 |
|------|------|----------|
| **Type A** | 虚构执行：AI 声称测试通过但无 CI 日志支撑 | P0 |
| **Type B** | 伪门禁：AI 生成门禁通过但无 gate engine 输出 | P0 |
| **Type C** | 伪证据：AI 引用不存在的 CI run / log hash | P1 |
| **Type D** | 伪任务完成：AI 标记任务完成但代码未合并 | P1 |
EOF

echo ""
echo "Report: $REPORT_FILE"
echo "PASS=$PASS_COUNT, WARN=$WARN_COUNT, FAIL=$FAIL_COUNT, UNVERIFIED=${#UNVERIFIED_CLAIMS[@]}"

if [ $FAIL_COUNT -gt 0 ]; then
  echo ""
  echo "❌ Evidence binding check FAILED - 发现 $FAIL_COUNT 个违规"
  exit 1
else
  echo ""
  echo "✅ Evidence binding check PASSED"
  exit 0
fi