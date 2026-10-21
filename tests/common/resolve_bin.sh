#!/usr/bin/env bash
#
# tests/common/resolve_bin.sh
#
# 统一的「外部可执行文件定位」—— #5406 / #5477。
#
# 用法：
#   source "$(dirname "${BASH_SOURCE[0]}")/resolve_bin.sh"
#   BIN="$(resolve_sqlrustgo_bin)" || exit 1
#
# 定位顺序（从最具体到最宽泛）：
#
#   1. 显式环境变量（SQLRUSTGO_BIN / SQLRUSTGO_CLI）
#   2. $CARGO_TARGET_DIR/{debug,release}/<name>   —— cargo 真正放产物的地方
#   3. <repo>/target/{debug,release}/<name>
#   4. $PATH
#
# **任何一步都不得回退到某个开发机的家目录路径。** 家目录路径是「这台机器
# 专属」的证据 —— 它出现在脚本里，那份脚本就只在这台机器上有效。
#
# 为什么「找不到」必须是非零退出码
# --------------------------------
#
# 这些脚本此前在二进制缺失时打印 `SKIP: ...` 然后 **exit 0**。
# exit 0 会被 CI / 调用方读成「这个回归测试通过了」—— 但它其实
# **一次都没跑**。这是 #5406 记录的同一类假绿，只是发生在 shell 层。
#
# 现在找不到二进制时：
#
#   * 打印 `TEST-NOT-RUN: <target> reason=binary_not_found ...`
#   * 以 **退出码 77**（automake 约定的 "skipped"）结束
#
# 77 既非 0（不是通过）也非 1（不是失败），调用方可以明确区分
# 「跑过了并通过」/「跑过了并失败」/「没跑」三态。

# 退出码：automake / GNU 测试框架约定的 "skipped"。
readonly TEST_SKIP_EXIT_CODE=77

# NOT-RUN 标记前缀。CI 按此前缀统计，并把它们从 passed 里扣掉。
readonly TEST_NOT_RUN_MARKER="TEST-NOT-RUN"

# 仓库根目录（本文件在 tests/common/ 下）。
resolve_bin_repo_root() {
  cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd
}

# 打印「找不到二进制」的三态披露并返回 skip 退出码。
#
# $1 = 测试目标名（用于 NOT-RUN 标签）
# $2 = 二进制名
# $3 = 应当设置的环境变量名
_test_bin_not_found() {
  local target="$1" name="$2" env_var="$3"
  echo "${TEST_NOT_RUN_MARKER}: ${target} reason=binary_not_found detail=${name} not found — set ${env_var} to its absolute path, or build it first (cargo build -p sqlrustgo-cli)" >&2
  return "${TEST_SKIP_EXIT_CODE}"
}

# 定位 sqlrustgo CLI。
#
# 优先 `SQLRUSTGO_BIN`，其次 `SQLRUSTGO_CLI`（历史名字），
# 再按 CARGO_TARGET_DIR / 仓库 target/ / PATH 找。
#
# #5405 — 候选名**只有** `sqlrustgo`
#
# `sqlrustgo-cli` 是**包名**（`crates/sqlrustgo-cli`），不是二进制名：
# 该包的 `[[bin]] name` 是 `sqlrustgo`。把它列为候选名，解析器就多走一条
# 永远落空的路径，并让「这个二进制叫什么」出现第二个来源。
# 权威的候选名由 `tests/common/bin_resolver.rs` 持有（Rust 侧），
# 本脚本是它在 shell 侧的等价物 —— 两侧必须一致。
#
# 找不到时**返回非零**（77），并打印 NOT-RUN 披露。
resolve_sqlrustgo_bin() {
  local target="${1:-$(basename "${0}")}"
  local candidate

  for env_var in SQLRUSTGO_BIN SQLRUSTGO_CLI; do
    if [ -n "${!env_var:-}" ]; then
      candidate="${!env_var}"
      if [ -x "$candidate" ]; then
        printf '%s\n' "$candidate"
        return 0
      fi
    fi
  done

  local repo_root
  repo_root="$(resolve_bin_repo_root)"
  local bases=()
  if [ -n "${CARGO_TARGET_DIR:-}" ]; then
    bases+=("$CARGO_TARGET_DIR")
  fi
  bases+=("$repo_root/target")

  for base in "${bases[@]}"; do
    for profile in debug release; do
      candidate="$base/$profile/sqlrustgo"
      if [ -x "$candidate" ]; then
        printf '%s\n' "$candidate"
        return 0
      fi
    done
  done

  # PATH
  #
  # 注意：`read -a` 在输入为空时**不会**给数组赋值，
  # 于是 `set -u` 下 `"${_path_dirs[@]}"` 会报「unbound variable」。
  # 所以先显式初始化为空数组。
  local dir
  local _path_dirs=()
  IFS=':' read -ra _path_dirs <<< "${PATH:-}"
  for dir in "${_path_dirs[@]}"; do
    [ -n "$dir" ] || continue
    for name in sqlrustgo; do
      candidate="$dir/$name"
      if [ -x "$candidate" ]; then
        printf '%s\n' "$candidate"
        return 0
      fi
    done
  done

  _test_bin_not_found "$target" "sqlrustgo" "SQLRUSTGO_BIN"
}
