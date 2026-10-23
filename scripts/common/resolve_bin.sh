#!/usr/bin/env bash
#
# scripts/common/resolve_bin.sh — scripts/ 侧的「二进制定位」入口。
#
# #5516：`scripts/` 下有 6 处脚本把**包名**当成**二进制名**（`sqlrustgo-cli`、
# `sqlrustgo-tools`、以及一个从未存在过的 `tpch_runner`），于是它们要么跑不到
# 东西，要么把「找不到」记成「通过」。
#
# 查找规则**只实现一次**，在 `tests/common/resolve_bin.sh` 里
# （`resolve_cargo_bin`）：那个文件是仓库里 shell 侧定位 cargo 产物的**唯一**
# 实现 —— 权威的「包名 → 二进制名」映射由 `scripts/gate/check_cli_binary_names.py`
# 持有。本文件只是 scripts/ 的入口，不复制任何查找逻辑。
#
# 用法：
#   SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
#   REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
#   source "$REPO_ROOT/scripts/common/resolve_bin.sh"
#
#   BIN="$(resolve_cargo_bin sqlrustgo SQLRUSTGO_BIN)" || exit $?
#
# 找不到时 `resolve_cargo_bin` 返回 **77**（automake 的 "skipped"）并打印
# `TEST-NOT-RUN:` 披露 —— 调用方**必须**把它当成「没跑」，不能当成通过。

# shellcheck source=../../tests/common/resolve_bin.sh
source "${REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}/tests/common/resolve_bin.sh"
