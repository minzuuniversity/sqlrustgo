#!/usr/bin/env bash
#
# scripts/ci/prepare_tpch_fixture_ci.sh — CI 侧的 TPC-H fixture 准备步骤
#
# WHY THIS FILE EXISTS
# ---------------------
#
# #5567: 两个 CI job（根包的 `test` 与成员包的 `test-crates`）都需要在跑
# 测试之前生成 TPC-H fixture。原来这件事直接写在 `.gitea/workflows/ci.yml`
# 的两个 step 里，各写一遍：
#
#     bash scripts/gate/prepare_tpch_fixture.sh --out ... --sf 0.01 --seed 42
#     echo "TPCH_DATA_DIR=..." >> "$GITHUB_ENV"
#     echo "TPCH_NOT_RUN_FILE=..." >> "$GITHUB_ENV"
#
# act_runner 用 `bash --noprofile --norc -e -o pipefail {0}` 跑每个 step，
# 所以生成器一旦失败，**整个 step 直接红**。而失败原因常常是**环境**的
# （runner 上没有 python3、/tmp 不可写、磁盘满），不是代码的 —— 环境阻塞
# 被报成了代码失败。
#
# 更糟的是：这个 step 一红，**紧邻的 NOT RUN 通道根本没机会生效**：
# `TPCH_NOT_RUN_FILE` 是给「跑不了的目标」用的（目标自己写
# `TPCH-NOT-RUN:` 记录，汇总 step 把它们从 passed 里扣掉），而 fixture
# 在目标开始跑之前就没了，那些目标压根没机会写记录。
#
# 所以本脚本把这件事收敛到**一处**（一个规则只允许有一份实现），并且
# 明确区分两类失败：
#
#   * **环境前提不满足**（没有 python3 / 输出目录不可写）⇒ 记 NOT RUN，
#     写进 `TPCH_NOT_RUN_FILE`，step 以 0 退出。
#   * **前提齐了仍然失败** ⇒ 生成器本身出了问题（代码回归）⇒ **必须红**。
#     绝不把「生成器坏了」放宽成「跑过了」。
#
# NOT RUN ≠ PASS：记录用与目标相同的 `TPCH-NOT-RUN:` 前缀写入
# `TPCH_NOT_RUN_FILE`，汇总 step 会把它从 passed 里扣掉，并单独披露。
# （口径见 `tests/common/tpch_fixture.rs` 的 `NOT_RUN_MARKER`。）
#
# Usage:
#   bash scripts/ci/prepare_tpch_fixture_ci.sh [--out <DIR>] [--sf <F>] [--seed <N>]
#
# Exit codes:
#   0 — fixture 就绪，或环境前提不满足（已记 NOT RUN）
#   2 — 用法错误
#   1 — 生成器在环境前提满足时失败（代码回归，必须红）

set -uo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
GENERATOR_WRAPPER="${REPO_ROOT}/scripts/gate/prepare_tpch_fixture.sh"

OUT_DIR="${RUNNER_TEMP:-/tmp}/tpch-fixture"
SF="0.01"
SEED="42"

# NOT RUN 的 reason 码与 `tests/common/tpch_fixture.rs` 的 reason 码表同源：
# 这里新增的两个码描述的是**环境**层面的阻塞，目标的 `fixture_missing`
# 描述的是**目标**层面的阻塞。两者不冲突：前者说「fixture 根本没生成」，
# 后者说「这个目标要 fixture 而 fixture 不在」。
REASON_PYTHON_MISSING="python3_missing"
REASON_DIR_NOT_WRITABLE="fixture_dir_not_writable"

usage() {
  cat >&2 <<'EOF'
Usage: bash scripts/ci/prepare_tpch_fixture_ci.sh [--out <DIR>] [--sf <F>] [--seed <N>]

  --out <DIR>    fixture 输出目录（默认 ${RUNNER_TEMP:-/tmp}/tpch-fixture）
  --sf <FLOAT>   TPC-H scale factor（默认 0.01）
  --seed <INT>   生成器随机种子（默认 42）
EOF
}

need_value() {
  [ "$1" -gt 0 ] || { echo "FATAL: $2 requires a value" >&2; exit 2; }
}

# 把 fixture 位置与 NOT RUN 记录位置交给后面的 step。本地跑（没有
# GITHUB_ENV）时静默跳过。
export_env() {
  if [ -n "${GITHUB_ENV:-}" ]; then
    printf 'TPCH_DATA_DIR=%s\n' "$OUT_DIR" >> "$GITHUB_ENV"
    printf 'TPCH_NOT_RUN_FILE=%s\n' "$NOT_RUN_FILE" >> "$GITHUB_ENV"
  fi
}

# 把 NOT RUN 记到目标用的同一个文件里（追加，不覆盖）。格式与
# `tests/common/tpch_fixture.rs` 的 `report_not_run` 一致：
#   TPCH-NOT-RUN: <target> reason=<code> detail=<...>
record_not_run() {
  local reason="$1"
  local detail="$2"
  printf 'TPCH-NOT-RUN: tpch_fixture reason=%s detail=%s\n' "$reason" "$detail" \
    | tee -a "$NOT_RUN_FILE"
}

while [ $# -gt 0 ]; do
  case "$1" in
    --out)   need_value $# "--out"; OUT_DIR="$2"; shift 2 ;;
    --out=*) OUT_DIR="${1#--out=}"; shift ;;
    --sf)    need_value $# "--sf"; SF="$2"; shift 2 ;;
    --sf=*)  SF="${1#--sf=}"; shift ;;
    --seed)  need_value $# "--seed"; SEED="$2"; shift 2 ;;
    --seed=*) SEED="${1#--seed=}"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "FATAL: unknown argument: $1" >&2; usage; exit 2 ;;
  esac
done

if [ ! -f "$GENERATOR_WRAPPER" ]; then
  echo "FATAL: generator wrapper not found: $GENERATOR_WRAPPER" >&2
  exit 1
fi

NOT_RUN_FILE="${RUNNER_TEMP:-/tmp}/tpch_not_run.log"
NOT_RUN_DIR="$(dirname "$NOT_RUN_FILE")"
[ -d "$NOT_RUN_DIR" ] || mkdir -p "$NOT_RUN_DIR" 2>/dev/null || true

# 环境前提 1：python3。生成器只用 Python 标准库（argparse / random / re /
# sys / datetime / pathlib），所以「python3 在不在 PATH 上」就是全部前提。
# 这一行与 `scripts/gate/prepare_tpch_fixture.sh` 里的同名检查重复 —— 那
# 一处是为了**发现**问题，这一处是为了**分类**问题（环境的还是代码的）。
if ! command -v python3 >/dev/null 2>&1; then
  echo "TPC-H fixture: NOT RUN — python3 is not on PATH on this runner."
  echo "STATUS: NOT RUN (not PASS, not FAIL)"
  record_not_run "$REASON_PYTHON_MISSING" "python3 is not on PATH"
  export_env
  exit 0
fi

# 环境前提 2：输出目录可写。磁盘满、/tmp 只读、目录建不出来，都在这里
# 被挡住，而不是留到生成器里炸。
if ! mkdir -p "$OUT_DIR" 2>/dev/null; then
  echo "TPC-H fixture: NOT RUN — cannot create the fixture directory $OUT_DIR."
  echo "STATUS: NOT RUN (not PASS, not FAIL)"
  record_not_run "$REASON_DIR_NOT_WRITABLE" "cannot create $OUT_DIR"
  export_env
  exit 0
fi
if ! touch "$OUT_DIR/.ci_writability_probe" 2>/dev/null; then
  echo "TPC-H fixture: NOT RUN — the fixture directory $OUT_DIR is not writable."
  echo "STATUS: NOT RUN (not PASS, not FAIL)"
  record_not_run "$REASON_DIR_NOT_WRITABLE" "$OUT_DIR is not writable"
  export_env
  exit 0
fi
rm -f "$OUT_DIR/.ci_writability_probe" 2>/dev/null || true

# 环境前提齐了。从这里开始的任何失败都是**生成器自身**的问题（代码回归），
# 必须把 step 打红 —— 不放宽、不降级成 NOT RUN。
if ! bash "$GENERATOR_WRAPPER" --out "$OUT_DIR" --sf "$SF" --seed "$SEED"; then
  echo "FATAL: the TPC-H fixture generator failed while the environment prerequisites" >&2
  echo "       were satisfied — that is a code regression, not an environment block." >&2
  export_env
  exit 1
fi

export_env
exit 0
