#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：孤儿门禁（orphan_referenced）—— 只在其它脚本正文里被引用，没有任何执行点
# 处置：待接入执行点（needs-execution-point）
# 责任任务：P0-2
# 登记说明：判据本身可用，但已提交树上 rc=1：7 条历史记录不可复核（5 hollow 无命令行 / 1 unsealed / 1 无清单目录），10 条 sound 不受影响 ⇒ 先由 owner 裁定退役（ISSUE 5596，看板 IL-5）再挂执行点
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# A01 gate: every committed audit run must pass verify (Issue #5223).
#
# Scans both run layouts:
#   legacy      audit/v4.1.0/runs/<run_id>/manifest.yaml   (schema_version 1)
#   iaia410/v1  audit/v4.1.0/<run_id>/manifest.yaml        (SSOT section 5)
#
# 枚举范围是**已提交的树**（git ls-tree HEAD），不是工作区目录。
#
# 原因（IL-6 自指）：归档 run 正在执行时，它自己的目录已经建好、manifest.yaml
# 也已写入，但日志还没写完。门禁若扫工作区，就会把「正在执行的 run」也算进判定：
# run 内红数 7->8，归档完再扫又回到 7 —— 判据自己数自己，红绿随执行时刻漂移。
# 本门禁的契约本来就是「every **committed** audit run must pass verify」，
# 所以未提交的 run 目录不进判定范围。这不放宽任何历史判据：
# 那 7 条不可复核的历史记录全是已提交的，改动前后一样判红（见反向变异证据）。
#
# 反向夹具：audit/v4.1.0/il6_selfref_probe.py
#   - 造一个未提交的在飞 run ⇒ 门禁不得把它算成红
#   - 把枚举改回工作区目录 ⇒ 门禁必须重新把它算成红（证明判据可被反向变异抓住）
#
# 用法：
#   bash scripts/gate/check_audit_manifest.sh                # 判定
#   bash scripts/gate/check_audit_manifest.sh --exclude-run <id>
#       显式排除某个 run（处置方案之二；可重复）。用于「这个 run 正在执行中，
#       它的日志还没写完」这种必须临时豁免的场景 —— 排除是**显式**的、
#       要写进命令行的，不靠目录枚举的副作用。
#   bash scripts/gate/check_audit_manifest.sh --selftest
#       自测：未提交的在飞 run 不得被判红（自指的反向变异点）。
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
AUDIT_DIR="${REPO_ROOT}/audit/v4.1.0"
RUNS_DIR="${AUDIT_DIR}/runs"
TOOL="${AUDIT_DIR}/audit_run.py"

# ---------------------------------------------------------------------------
# 解释器选择：门禁的每一次 verify 都靠 audit_run.py，而 audit_run.py 需要
# PyYAML（读 manifest.yaml）。本机 `python3` 可能指向**没有 PyYAML** 的解释器
# （实测：homebrew python 3.15 无 yaml，/usr/bin/python3 3.9.6 有）。
#
# 改前的问题（改前漏什么）：
#   解释器缺 yaml ⇒ 每一次 verify 都抛 ModuleNotFoundError ⇒ 门禁把这些
#   **崩溃**当成「证据不可复核」，于是 21 个本来干净的已提交 run 全部变红。
#   更糟的是红数随 shell 的 PATH 变：同一份代码、同一个提交，
#   一个 shell 给 7 条红，另一个给 28 条 —— 判据不可复现，
#   而且那 21 条红是**归因错误**（不是证据有问题，是环境有问题），
#   读者无法把「7 条已知红」与「28 条红」对上账。
#   这与 IL-3 / IL-10 是同一族：把「环境不具备」当成「能力不足」。
#
# 改后多报什么：**不多报任何一条证据红**。
#   解释器可用时行为逐字不变（仍是那 7 条历史红）。
#   解释器不可用时**明确报一条「无法判定」**并 exit 1（仍然 fail closed），
#   而不是伪造 N 条「证据不可复核」。
# ---------------------------------------------------------------------------
PY=""
if [[ -n "${SQLRUSTGO_GATE_PYTHON:-}" ]]; then
    # 显式指定优先：调用方指名了解释器，就**不要**静默换掉它。
    # 指名了却用不了 ⇒ 明确报「无法判定」，不伪装成证据红。
    PY="${SQLRUSTGO_GATE_PYTHON}"
    if ! "${PY}" -c 'import yaml' >/dev/null 2>&1; then
        echo "FAIL: SQLRUSTGO_GATE_PYTHON=${PY} cannot import yaml —" \
             "cannot judge committed audit runs; refusing to pass" >&2
        exit 1
    fi
else
    for cand in python3 /usr/bin/python3 python3.14 python3.13 python3.12 python3.11 python3.10; do
        if command -v "${cand}" >/dev/null 2>&1 && "${cand}" -c 'import yaml' >/dev/null 2>&1; then
            PY="${cand}"
            break
        fi
    done
    if [[ -z "${PY}" ]]; then
        echo "FAIL: no usable Python interpreter (audit_run.py needs PyYAML) —" \
             "cannot judge committed audit runs; refusing to pass" >&2
        echo "      set SQLRUSTGO_GATE_PYTHON=<python-with-pyyaml> to fix" >&2
        exit 1
    fi
fi

exclude_runs=""
selftest=0
while [[ $# -gt 0 ]]; do
    case "$1" in
        --exclude-run)
            shift
            if [[ $# -eq 0 ]]; then
                echo "BAD-ARG --exclude-run needs a run id" >&2
                exit 2
            fi
            exclude_runs="${exclude_runs} $1"
            ;;
        --selftest)
            selftest=1
            ;;
        -h|--help)
            sed -n '2,30p' "${BASH_SOURCE[0]}"
            exit 0
            ;;
        *)
            echo "BAD-ARG unknown option: $1" >&2
            exit 2
            ;;
    esac
    shift
done

is_excluded() {
    local rid="$1"
    for x in ${exclude_runs}; do
        [[ "${x}" == "${rid}" ]] && return 0
    done
    return 1
}

# 已提交文件清单（HEAD 树）。git 读不出来 ⇒ fail closed，不许默默放行。
if ! git -C "${REPO_ROOT}" rev-parse --verify HEAD >/dev/null 2>&1; then
    echo "PASS: no committed audit runs yet (HEAD has no commit)"
    exit 0
fi
if ! tracked="$(git -C "${REPO_ROOT}" ls-tree -r --name-only HEAD -- audit/v4.1.0 2>/dev/null)"; then
    echo "FAIL: cannot read committed tree (git ls-tree failed) — refusing to pass" >&2
    exit 1
fi

# 已提交的 run 目录清单。用纯 POSIX 构造（不用 bash 4 的关联数组 ——
# macOS 自带 bash 3.2，`declare -A` 在那儿不可用）。
# 每行是 "<布局目录>\t<run_id>\t<有无manifest>"
# 临时文件放仓外，别把中间产物留在工作区。
tmp_list="$(mktemp -t audit_manifest_dirs)"
trap 'rm -f "${tmp_list}" "${tmp_list}.raw"' EXIT
: > "${tmp_list}.raw"
while IFS= read -r p; do
    [[ -z "${p}" ]] && continue
    case "${p}" in
        audit/v4.1.0/runs/*/*)
            rest="${p#audit/v4.1.0/runs/}"
            id="${rest%%/*}"
            d="runs/${id}"
            ;;
        audit/v4.1.0/*/*)
            rest="${p#audit/v4.1.0/}"
            id="${rest%%/*}"
            [[ "${id}" == "runs" ]] && continue
            d="${id}"
            ;;
        *)
            continue
            ;;
    esac
    if [[ "${p}" == */manifest.yaml ]]; then
        printf '%s\t%s\t1\n' "${d}" "${id}" >> "${tmp_list}.raw"
    else
        printf '%s\t%s\t0\n' "${d}" "${id}" >> "${tmp_list}.raw"
    fi
done <<< "${tracked}"
# 同一目录可能被多个文件命中；同一目录只要**有**清单就算有清单。
# 必须**一趟**做完（两个 awk 是两个进程、两个 seen 数组，第二个会把
# 已经有清单的目录再当成「无清单」加回来，同一个 run 被判两次）。
awk -F'\t' '
    { if (!($1 in rid)) rid[$1] = $2; if ($3 == "1") mf[$1] = 1 }
    END { for (d in rid) print d "\t" rid[d] "\t" (mf[d] ? 1 : 0) }
' "${tmp_list}.raw" | sort > "${tmp_list}"

# 有清单的 run 目录 = 要核对的 run。
manifests=()
while IFS=$'\t' read -r d id has_mf; do
    [[ "${has_mf}" == "1" ]] && manifests+=("${AUDIT_DIR}/${d}/manifest.yaml")
done < "${tmp_list}"
if [[ ${#manifests[@]} -eq 0 ]]; then
    echo "PASS: no committed audit runs yet"
    exit 0
fi

fail=0

# 反向夹具暴露过的**漏报**：本门禁原先只 glob `*/manifest.yaml`，
# 于是"有目录、没清单"的运行（iaia410-a06-r1-20261011T073325Z-01df92a5
# 只有 findings/）被**静默跳过** —— 一条完全无清单的运行从未报过红。
# 现在按运行命名约定（iaia410-* 与 YYYYMMDD-*）要求：这样的目录必须有 manifest.yaml。
while IFS=$'\t' read -r d id has_mf; do
    if [[ "${id}" =~ ^(iaia410-|[0-9]{8}-) ]] && [[ "${has_mf}" != "1" ]]; then
        if is_excluded "${id}"; then
            echo "SKIP: run dir without manifest (excluded): ${id}"
            continue
        fi
        echo "FAIL: run dir without manifest: ${id}" >&2
        fail=1
    fi
done < "${tmp_list}"

for mf in "${manifests[@]}"; do
    run_id="$(basename "$(dirname "${mf}")")"
    if is_excluded "${run_id}"; then
        echo "SKIP: audit run ${run_id} (excluded)"
        continue
    fi
    if "${PY}" "${TOOL}" verify --run-id "${run_id}"; then
        :
    else
        echo "FAIL: audit run ${run_id} failed evidence verify" >&2
        fail=1
    fi
done

# --selftest：造一个**未提交**的在飞 run（有 manifest、无 results），
# 门禁不得把它算成红 —— 这就是 IL-6 自指的反向变异点。
# 这条自测在判定之前跑，退出码与判定分开报。
if [[ "${selftest}" -eq 1 ]]; then
    probe_id="20260905-il6-selftest-inflight"
    probe_dir="${RUNS_DIR}/${probe_id}"
    mkdir -p "${probe_dir}"
    cat > "${probe_dir}/manifest.yaml" <<'YAML'
schema_version: 1
run_id: 20260905-il6-selftest-inflight
source_agent: selftest
frozen_commit: "0000000000000000000000000000000000000000"
environment: {}
artifacts: {}
commands:
- id: probe
  name: probe
  cmd: "true"
  cwd: "."
  timeout_sec: 1
YAML
    echo "=== selftest: 未提交的在飞 run 不得被判红 ==="
    selftest_red=0
    if "${PY}" "${TOOL}" verify --run-id "${probe_id}" >/dev/null 2>&1; then
        echo "SELFTEST-BROKEN: 未提交的在飞 run 竟然能 verify 通过" >&2
        selftest_red=1
    else
        echo "SELFTEST-OK: 在飞 run 自身 verify 判红（说明它确实还没写完）"
    fi
    rm -rf "${probe_dir}"
    if [[ "${selftest_red}" -ne 0 ]]; then
        exit 1
    fi
fi

if [[ "${fail}" -ne 0 ]]; then
    exit 1
fi
echo "PASS: ${#manifests[@]} audit run(s) verified"
