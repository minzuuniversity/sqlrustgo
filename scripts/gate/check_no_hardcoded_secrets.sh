#!/usr/bin/env bash
#
# check_no_hardcoded_secrets.sh — 禁止明文凭据进入可执行代码
# ==============================================================================
# 背景（#5381）:
#   仓库内曾硬编码可用的 Gitea 口令与 token。经实测这些凭据**仍然有效**
#   ——仓库读权限 == Gitea 写权限。涉及可执行脚本 6 个、文档 13 个文件。
#
# 本门禁扫描**可执行代码**（scripts/ tools/ config/ .gitea/），文档不扫:
#   历史计划与证据文档里的凭据需要人工逐条改写并保留证据链，
#   清单在 #5381 里，不适合用一条正则粗暴处理。
#
# 检测的是凭据的**形状**而不是具体字面量——检测器本身绝不能把密钥写进仓库。
# 四条独立的扫描，每条都刻意收窄，避免把 git SHA、"PASS:" 这类状态词误报。
#
# 退出: 0=PASS, 1=FAIL
# ==============================================================================

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT" || exit 1

ERRORS=0
SELF="scripts/gate/check_no_hardcoded_secrets.sh"

SCAN_PATHS=()
for d in scripts tools config .gitea; do
    [ -d "$d" ] && SCAN_PATHS+=("$d")
done

if [ "${#SCAN_PATHS[@]}" -eq 0 ]; then
    echo "PASS: 没有可扫描的代码目录"
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

echo "=== 硬编码凭据扫描 (形状匹配, 非字面量) ==="

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

# ── 扫描 2: 已观察到的口令形状 ───────────────────────────────────────────
hits=$(scan_i 'details[0-9]{4}')
if [ -n "${hits}" ]; then
    echo ""
    echo "FAIL — 发现口令字面量:"
    echo "${hits}" | sed 's/^/  /'
    ERRORS=$((ERRORS + 1))
else
    echo "PASS: 没有口令字面量"
fi

# ── 扫描 3: curl 基本认证里出现字面量凭据 ────────────────────────────────
# 只在 curl 行上找 -u "user:pass"，且 pass 里不能有 $（排除 ${VAR} 形式）。
hits=$(scan 'curl' | grep -E '\-u[[:space:]]+["'"'"'][^"'"'"'$]*:[^"'"'"'$]{4,}["'"'"']' || true)
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
hits=$(scan '://[^/[:space:]]*:[^/@[:space:]]{4,}@')
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
    echo "             严禁静默回落到任何写死的值。详见 #5381。"
    exit 1
fi
echo "✅ 硬编码凭据检查通过"
exit 0
