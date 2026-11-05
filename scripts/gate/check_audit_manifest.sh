#!/usr/bin/env bash
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
    if python3 "${TOOL}" verify --run-id "${run_id}"; then
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
    if python3 "${TOOL}" verify --run-id "${probe_id}" >/dev/null 2>&1; then
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
