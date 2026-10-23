#!/usr/bin/env bash
# scripts/sync/gitea_credential_common.sh — Gitea 凭据解析的**唯一**实现。
#
# WHY THIS FILE EXISTS
# ---------------------
#
# 三个仓库脚本需要 Gitea 凭据（`5remotes_sync.sh`、`push_q21_merge_to_gitea.sh`、
# `sync_252_when_ready.sh`）。它们过去各自内嵌了一份明文口令/token ——
# 那两个值是**活的**，并因此被当作泄露跟踪（#5381）。
#
# 「一个规则只能有一份实现」：凭据从哪来、凭什么取不到时该失败、
# 以及**绝不把它写进 URL** —— 这三条规则只在这里定义一次，
# 调用方 `source` 本文件，因而不可能互相漂移。
#
# THE RULES
# ---------
#
#   C1  来源优先级：**环境变量** `GITEA_USER` / `GITEA_TOKEN`
#       > 本机 **git credential store**（`git credential fill`）。
#       环境变量优先，因为 CI 里通常只能通过环境注入。
#
#   C2  凭据**绝不**出现在 URL 里（`http://user:pass@host` 会进
#       `.git/config`、进程列表和 CI 日志）。git 侧用
#       `git -c http.extraheader="Authorization: Basic …"`，
#       curl 侧用 `-H`。
#
#   C3  取不到凭据时**明确失败**（返回非零），
#       **绝不**回落到任何写死的值。
#
#   C4  本文件的任何函数都**不得**把凭据打印到 stdout/stderr。
#       需要凭据的调用方必须自己捕获输出。
#
# Usage:
#   source "$(dirname "${BASH_SOURCE[0]}")/gitea_credential_common.sh"
#   gitea_credential_resolve "192.168.0.252:3000" || exit 2
#   # after a successful resolve: $GITEA_USER and $GITEA_TOKEN are set

# Read the credential store for one host.
#
# Prints the raw `git credential fill` output (username=… / password=…).
# The caller MUST capture it; printing it to a terminal would leak it.
# Returns 1 when the store has no usable entry.
gitea_credential_fill() {
    local host="${1:-}"
    [ -n "$host" ] || return 1
    local creds
    # GIT_TERMINAL_PROMPT=0: without it, `git credential fill` on a host with
    # no stored entry blocks on a terminal prompt — and this script runs
    # non-interactively (CI, cron, agent loops).
    creds=$(printf 'protocol=http\nhost=%s\n\n' "$host" \
        | GIT_TERMINAL_PROMPT=0 git credential fill 2>/dev/null) || creds=""
    [ -n "$creds" ] || return 1
    printf '%s\n' "$creds"
}

# Resolve the Gitea credential into GITEA_USER / GITEA_TOKEN.
#
#   $1  host:port to look up in the credential store (default: the 252 host)
#
# On success sets GITEA_USER and GITEA_TOKEN and returns 0.
# On failure returns 1 and leaves the variables untouched — the caller is
# expected to report the failure explicitly (C3).
gitea_credential_resolve() {
    local host="${1:-192.168.0.252:3000}"
    if [ -z "${GITEA_USER:-}" ] || [ -z "${GITEA_TOKEN:-}" ]; then
        local creds u p
        creds="$(gitea_credential_fill "$host")" || creds=""
        if [ -n "$creds" ]; then
            u=$(printf '%s\n' "$creds" | sed -n 's/^username=//p' | head -1)
            p=$(printf '%s\n' "$creds" | sed -n 's/^password=//p' | head -1)
            [ -n "${GITEA_USER:-}" ] || GITEA_USER="$u"
            [ -n "${GITEA_TOKEN:-}" ] || GITEA_TOKEN="$p"
        fi
    fi
    [ -n "${GITEA_USER:-}" ] && [ -n "${GITEA_TOKEN:-}" ]
}

# Build the Basic auth header value for the resolved credential.
#
#   $1  header name (default: "Authorization: Basic …")
#
# The credential is base64-encoded, so the header still must not be printed.
gitea_credential_basic_header() {
    [ -n "${GITEA_USER:-}" ] && [ -n "${GITEA_TOKEN:-}" ] || return 1
    printf 'Authorization: Basic %s' \
        "$(printf '%s:%s' "${GITEA_USER}" "${GITEA_TOKEN}" | base64 | tr -d '\n')"
}

# ── 「凭据不进进程列表」的两条通道 ─────────────────────────────────────────
#
# C2 只说了凭据不能进 **URL**。但 `curl -H "Authorization: …"` 与
# `git -c http.extraheader=…` 都会把凭据放进 **argv** ⇒ `ps aux` 就能读到它，
# CI 的进程列表、崩溃报告、审计日志同样会留下它。
#
# 下面两条通道让 argv 里只剩一个**路径**：
#
#   curl → gitea_credential_curl_config_file
#          用法: `curl -K "$file" …`（-K 读一个 curl 配置文件）
#   git  → gitea_credential_git_config_file
#          用法: `git -c include.path="$file" …`（include.path 引入一个 git 配置文件）
#
# 两条都不写进 `.git/config`（配置文件在仓库外、用完即删），
# 也不出现在命令行上。两个文件都是 0600。
#
# base64 之后不含引号/反斜杠 ⇒ 直接写进配置文件是安全的。

# Write a 0600 curl config file carrying the auth header; print its path.
# The caller owns the file and MUST delete it (the scripts trap EXIT).
gitea_credential_curl_config_file() {
    local file header
    [ -n "${GITEA_USER:-}" ] && [ -n "${GITEA_TOKEN:-}" ] || return 1
    header="$(gitea_credential_basic_header)" || return 1
    file="$(mktemp "${TMPDIR:-/tmp}/gitea-curl-auth.XXXXXX")" || return 1
    chmod 600 "$file" 2>/dev/null || { rm -f "$file"; return 1; }
    printf 'header = "%s"\n' "$header" > "$file" || { rm -f "$file"; return 1; }
    printf '%s\n' "$file"
}

# Write a 0600 git config file carrying http.extraheader; print its path.
# The caller owns the file and MUST delete it (the scripts trap EXIT).
gitea_credential_git_config_file() {
    local file header
    [ -n "${GITEA_USER:-}" ] && [ -n "${GITEA_TOKEN:-}" ] || return 1
    header="$(gitea_credential_basic_header)" || return 1
    file="$(mktemp "${TMPDIR:-/tmp}/gitea-git-auth.XXXXXX")" || return 1
    chmod 600 "$file" 2>/dev/null || { rm -f "$file"; return 1; }
    printf '[http]\n\textraheader = "%s"\n' "$header" > "$file" \
        || { rm -f "$file"; return 1; }
    printf '%s\n' "$file"
}
