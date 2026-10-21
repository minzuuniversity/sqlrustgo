#!/usr/bin/env bash
#
# check_no_hardcoded_secrets.sh — 禁止明文凭据进入仓库
# ==============================================================================
# 背景（#5381 / #5520）:
#   仓库内曾硬编码可用的 Gitea 口令与 token。经实测这些凭据**仍然有效**
#   ——仓库读权限 == Gitea 写权限。涉及可执行脚本 6 个、文档 13 个文件。
#
# #5520 的实测订正:
#   1. 那 13 个**文档**文件当时**没有被清理** —— 它们至今仍在 develop 的历史里，
#      而 develop/v4.1.0 在 **public** 的 GitHub 镜像
#      (yinglichina8848/sqlrustgo-public) 上 ⇒ **匿名可读**。
#      实测: 匿名 GET .../contents/docs/releases/v3.12.0/SCOPE_TABLE_v3.12.md
#      直接返回了明文口令。
#      ⇒ 本门禁现在**也扫文档**（文档已在本批次完成清洗）。
#   2. 明文凭据还留在**别的分支**上（gitcode 的公开仓库里那条
#      fix/4597-chaos-t8d-awk-portability 分支）⇒ 门禁必须能对**任意
#      --repo-root** 运行，才能对别的分支做体检。
#
# 检测的是凭据的**形状**而不是具体字面量——检测器本身绝不能把密钥写进仓库。
# 每条扫描都刻意收窄，避免把 git SHA、"PASS:" 这类状态词误报。
#
# 退出: 0=PASS, 1=FAIL, 2=用法错误
#
# 用法:
#   bash scripts/gate/check_no_hardcoded_secrets.sh
#   bash scripts/gate/check_no_hardcoded_secrets.sh --selftest
#   bash scripts/gate/check_no_hardcoded_secrets.sh --repo-root <dir>
# ==============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT_OVERRIDE=""
SELFTEST=0

while [ $# -gt 0 ]; do
    case "$1" in
        --selftest)   SELFTEST=1; shift ;;
        --repo-root)  REPO_ROOT_OVERRIDE="${2:-}"; shift 2 ;;
        --repo-root=*) REPO_ROOT_OVERRIDE="${1#--repo-root=}"; shift ;;
        -h|--help)
            sed -n '2,/^set -uo/p' "$0" | sed '$d'
            exit 0
            ;;
        *) echo "ERROR: unknown argument: $1" >&2; exit 2 ;;
    esac
done

REPO_ROOT="${REPO_ROOT_OVERRIDE:-$(cd "${SCRIPT_DIR}/../.." && pwd)}"
cd "$REPO_ROOT" || exit 1

ERRORS=0
SELF="scripts/gate/check_no_hardcoded_secrets.sh"

SCAN_PATHS=()
for d in scripts tools config .gitea docs; do
    [ -d "$d" ] && SCAN_PATHS+=("$d")
done

if [ "${#SCAN_PATHS[@]}" -eq 0 ]; then
    echo "PASS: 没有可扫描的目录"
    exit 0
fi

# 通用过滤: 排除注释行与本门禁自身
scan() {
    grep -rnE "$1" "${SCAN_PATHS[@]}" --exclude="$(basename "${SELF}")" 2>/dev/null \
        | grep -vE "^${SELF}:" \
        | grep -vE ':[0-9]+:[[:space:]]*#' \
        || true
}

# 同上，但关键词大小写不敏感 (GITEA_TOKEN / Gitea_Token / gitea_token 都要覆盖)。
# BSD grep 的 -E 不支持 (?i) 修饰符，只能走 -i 开关。
scan_i() {
    grep -rniE "$1" "${SCAN_PATHS[@]}" --exclude="$(basename "${SELF}")" 2>/dev/null \
        | grep -vE "^${SELF}:" \
        | grep -vE ':[0-9]+:[[:space:]]*#' \
        || true
}

# ---------------------------------------------------------------------------
# --selftest: 门禁必须能失败 —— 注入已知形态，逐一验证被抓住
# ---------------------------------------------------------------------------
if [ "${SELFTEST}" -eq 1 ]; then
    echo "=== 自检: 注入已知泄露形态 ==="
    TMP="$(mktemp -d "${TMPDIR:-/tmp}/secrets-selftest.XXXXXX")"
    trap 'rm -rf -- "${TMP}"' EXIT

    mkdir -p "${TMP}/scripts"
    SELF_FAILURES=0

    probe() {
        # probe <名称> <期望 PASS|FAIL> <内容>
        local name="$1" expect="$2" body="$3"
        printf '%s\n' "${body}" > "${TMP}/scripts/probe.sh"
        local out status
        out=$(cd "${TMP}" && bash "${SCRIPT_DIR}/check_no_hardcoded_secrets.sh" \
              --repo-root "${TMP}" 2>&1)
        status=$?
        local got="PASS"
        [ "${status}" -ne 0 ] && got="FAIL"
        if [ "${got}" = "${expect}" ]; then
            printf '  PASS: %s (期望 %s，实际 %s)\n' "${name}" "${expect}" "${got}"
        else
            printf '  FAIL: %s (期望 %s，实际 %s)\n' "${name}" "${expect}" "${got}"
            SELF_FAILURES=$((SELF_FAILURES + 1))
        fi
    }

    # 1. 干净文件 ⇒ PASS（门禁不能空过）
    probe "干净脚本" "PASS" '#!/bin/bash
echo hello'

    # 2. 凭据名变量 ← 长十六进制字面量 ⇒ FAIL
    probe "凭据变量 ← 十六进制字面量" "FAIL" '#!/bin/bash
GITEA_TOKEN="0123456789abcdef0123456789abcdef01234567"'

    # 3. curl 基本认证里的字面量 ⇒ FAIL
    probe "curl 字面量基本认证" "FAIL" '#!/bin/bash
curl -s -u "someuser:s3cretpass" "http://example.invalid/api"'

    # 4. URL 内嵌凭据 ⇒ FAIL
    probe "URL 内嵌凭据" "FAIL" '#!/bin/bash
git clone "http://someuser:s3cretpass@example.invalid/repo.git"'

    # 5. 字面量的 名字:口令 对（带数字）⇒ FAIL
    probe "字面量 名字:口令 对" "FAIL" '#!/bin/bash
AUTH="someuser:s3cret8848"'

    # 6. 文档里的字面量也要被抓住（#5520 的实际漏网点）
    mkdir -p "${TMP}/docs"
    printf '%s\n' 'curl -s -u "someuser:s3cret8848" http://example.invalid' \
        > "${TMP}/docs/plan.md"
    out=$(cd "${TMP}" && bash "${SCRIPT_DIR}/check_no_hardcoded_secrets.sh" \
          --repo-root "${TMP}" 2>&1)
    status=$?
    if [ "${status}" -ne 0 ]; then
        printf '  PASS: 文档里的字面量也被抓住\n'
    else
        printf '  FAIL: 文档里的字面量被漏掉\n'
        SELF_FAILURES=$((SELF_FAILURES + 1))
    fi
    rm -f "${TMP}/docs/plan.md"

    # 7. 环境变量引用 ⇒ PASS（不得误伤正确的写法）
    probe "环境变量引用" "PASS" '#!/bin/bash
curl -s -u "${GITEA_USER}:${GITEA_TOKEN}" "http://example.invalid/api"'

    # 8. host:port ⇒ PASS（不得把端口当口令）
    probe "host:port" "PASS" '#!/bin/bash
URL="http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo"'

    # 9. git SSH 形态 ⇒ PASS
    probe "git@host SSH URL" "PASS" '#!/bin/bash
git clone "ssh://git@192.168.0.252:222/openclaw/sqlrustgo.git"'

    echo ""
    if [ "${SELF_FAILURES}" -eq 0 ]; then
        echo "✅ 自检通过"
        exit 0
    fi
    echo "❌ 自检失败: ${SELF_FAILURES} 项"
    exit 1
fi

echo "=== 硬编码凭据扫描 (形状匹配, 非字面量) ==="
echo "    扫描根目录: ${REPO_ROOT}"

# ── 扫描 1: 凭据名变量被赋了一个长十六进制字面量 ─────────────────────────
# 要求: 同行出现凭据关键词 + 至少 16 位十六进制 + 至少一个非零字符。
#   关键词要求 → git commit SHA（纯 40 位十六进制但无关键词）不算
#   长度要求 → "PASS: first_call..." 这类状态词不算
#   非零要求 → 全零占位符 (0000...0) 不算
hits=$(scan_i '(token|pass(word|wd)?|secret|api[_-]?key)[[:space:]]*[=:][[:space:]]*["'"'"']?[0-9a-f]{15,23}[1-9a-f][0-9a-f]{0,30}')
if [ -n "${hits}" ]; then
    echo "FAIL — 凭据名变量被赋了长十六进制字面量:"
    echo "${hits}" | sed 's/^/  /'
    ERRORS=$((ERRORS + 1))
else
    echo "PASS: 没有凭据名变量 ← 长十六进制字面量"
fi

# ── 扫描 2: 字面量的「名字:口令」对 ──────────────────────────────────────
# 出现在引号里，或紧跟 -u 之后。
#   「名字」部分必须**只有字母/下划线/连字符** —— 于是 "sha256:…" 这类
#   哈希前缀、"host:port" 这类地址都不会命中；
#   「口令」部分必须 ≥4 位 [A-Za-z0-9] 且**同时含字母和数字** —— 于是
#   TPC-H 那种 "customer:150000"（行数）不会命中，
#   而 "details8848" / "s3cret8848" 会命中；
#   含 $ 的 ${VAR} 形式被排除（那是正确写法）。
# 这一条不依赖任何具体字面量：#5520 里那条口令是「名字 + 4 位数字」的形状，
# 换成别的口令就漏了 —— 那样的门禁挡不住下一条泄漏。
hits=$(scan '(["'"'"']|-u[[:space:]])[A-Za-z][A-Za-z_-]{2,}:[A-Za-z0-9]{3,}[A-Za-z][A-Za-z0-9]*[0-9][A-Za-z0-9]*["'"'"']?' \
    || true)
if [ -n "${hits}" ]; then
    echo ""
    echo "FAIL — 发现字面量的「名字:口令」对:"
    echo "${hits}" | sed 's/^/  /'
    ERRORS=$((ERRORS + 1))
else
    echo "PASS: 没有字面量的「名字:口令」对"
fi

# ── 扫描 3: curl 基本认证里出现字面量凭据 ────────────────────────────────
# 只在 curl 行上找 -u "user:pass"，且 pass 里不能有 $（排除 ${VAR} 形式）
# 也不能是 <…> 占位符（那是**正确**的写法，#5520 的清洗就是这么做的）。
hits=$(scan 'curl' | grep -E '\-u[[:space:]]+["'"'"'][^"'"'"'$<]*:[^"'"'"'$<]{4,}["'"'"']' \
    | grep -vE ':(TOKEN|PASSWORD|PASSWD|SECRET|XXX+|<[^>]+>)["'"'"']?$' \
    || true)
if [ -n "${hits}" ]; then
    echo ""
    echo "FAIL — curl 基本认证使用了字面量凭据:"
    echo "${hits}" | sed 's/^/  /'
    ERRORS=$((ERRORS + 1))
else
    echo "PASS: curl 没有使用字面量凭据"
fi

# ── 扫描 4: URL 内嵌凭据 ─────────────────────────────────────────────────
# 嵌在 URL 里的凭据会进 .git/config、进程表与 CI 日志。
# 字面量与 ${VAR} 形式都算——后者同样会泄漏。
hits=$(scan '://[^/[:space:]]*:[^/@[:space:]<]{4,}@')
if [ -n "${hits}" ]; then
    echo ""
    echo "FAIL — 凭据被嵌进了 URL:"
    echo "${hits}" | sed 's/^/  /'
    echo "         改用 http.extraheader / Authorization 头传凭据。"
    ERRORS=$((ERRORS + 1))
else
    echo "PASS: 没有凭据被嵌进 URL"
fi

echo ""
if [ "${ERRORS}" -ne 0 ]; then
    echo "❌ 硬编码凭据检查未通过 (${ERRORS} 项)"
    echo "   修复方式: 改为从环境变量读取, 缺失即明确报错退出；"
    echo "             严禁静默回落到任何写死的值。详见 #5381 / #5520。"
    exit 1
fi
echo "✅ 硬编码凭据检查通过"
exit 0
