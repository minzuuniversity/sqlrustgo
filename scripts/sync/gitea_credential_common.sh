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
