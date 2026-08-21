#!/usr/bin/env bash
# setup-sqlrustgo-cli.sh
# 在 sqlrustgo 仓库内构建 sqlrustgo-cli / sqlrustgo-mysql-server 并做冒烟测试。
# 用于第一周上机: 环境准备 + 第一条 SQL 运行记录。
#
# 用法 (在仓库根目录下运行):
#   bash docs/teaching-labs/setup-sqlrustgo-cli.sh            # build (debug) + 冒烟
#   RELEASE=1 bash docs/teaching-labs/setup-sqlrustgo-cli.sh  # release 构建
#   SKIP_SMOKE=1 bash docs/teaching-labs/setup-sqlrustgo-cli.sh  # 跳过冒烟测试
#
# 说明: 本仓库根包 sqlrustgo 是库 (无 bin), 必须用 -p 显式选包。
# sqlrustgo-cli 是 sqlrustgo-mysql-server 的薄包装, 运行时通过子进程调用后者,
# 因此两个二进制必须同时构建, 且必须从仓库根目录运行 (以便找到 ./target/.../sqlrustgo-mysql-server)。

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET_DIR="target/debug"
if [ "${RELEASE:-0}" = "1" ]; then
  TARGET_DIR="target/release"
fi

log()  { printf '\033[1;34m[setup]\033[0m %s\n' "$*"; }
ok()   { printf '\033[1;32m[ ok ]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[warn]\033[0m %s\n' "$*"; }
err()  { printf '\033[1;31m[fail]\033[0m %s\n' "$*"; }

check_toolchain() {
  log "检查 Rust 工具链 ..."
  local missing=0
  for bin in rustc cargo git; do
    if command -v "$bin" >/dev/null 2>&1; then
      ok "$bin -> $("$bin" --version 2>&1 | head -1)"
    else
      err "$bin 未安装"
      missing=1
    fi
  done
  if ! command -v cc >/dev/null 2>&1; then
    err "C 链接器 (cc/gcc) 未安装, rusqlite bundled 编译需要它: sudo apt install build-essential"
    missing=1
  fi
  if [ "$missing" -ne 0 ]; then
    err "工具链不完整, 请先安装 Rust / Git / build-essential 后重试。"
    exit 1
  fi
}

build_bins() {
  cd "$REPO_ROOT" || { err "进入 $REPO_ROOT 失败"; return 1; }

  local profile_arg=""
  if [ "${RELEASE:-0}" = "1" ]; then
    profile_arg="--release"
  fi

  log "cargo build $profile_arg -p sqlrustgo-cli -p sqlrustgo-mysql-server ..."
  if cargo build $profile_arg -p sqlrustgo-cli -p sqlrustgo-mysql-server; then
    ok "cargo build 成功"
    ok "可执行程序: $REPO_ROOT/$TARGET_DIR/sqlrustgo-cli (CLI 薄包装)"
    ok "            $REPO_ROOT/$TARGET_DIR/sqlrustgo-mysql-server (实际执行入口)"
  else
    err "cargo build 失败 (常见: 依赖下载慢 -> 配置 rsproxy 镜像; rusqlite bundled 需要 build-essential)。"
    return 1
  fi
}

smoke_test() {
  if [ "${SKIP_SMOKE:-0}" = "1" ]; then
    warn "SKIP_SMOKE=1, 跳过冒烟测试。"
    return 0
  fi

  cd "$REPO_ROOT" || return 1
  local CLI="./$TARGET_DIR/sqlrustgo-cli"
  local SRV="./$TARGET_DIR/sqlrustgo-mysql-server"

  log "冒烟 1/3: sqlrustgo-cli exec 单条 SQL ..."
  if $CLI exec "SELECT 1" 2>&1 | sed 's/^/    /'; then
    ok "exec 单条 SQL 通过"
  else
    warn "exec 单条 SQL 失败 (exit $?)"
  fi

  log "冒烟 2/3: sqlrustgo-cli repl ..."
  local out2
  out2="$(printf 'SELECT 1;\n.exit\n' | timeout 15 $CLI repl 2>&1)"
  printf '%s\n' "$out2" | sed 's/^/    /'
  if printf '%s' "$out2" | grep -q 'Integer(1)'; then
    ok "repl 通过 (进入 REPL, SELECT 1 命中)"
  else
    warn "repl 未命中预期结果, 请人工核对"
  fi

  log "冒烟 3/3: sqlrustgo-mysql-server repl 多语句会话 (CREATE/INSERT/SELECT) ..."
  local sql_in='CREATE TABLE users (id INTEGER, name TEXT);
INSERT INTO users VALUES (1, '"'"'Ada'"'"');
INSERT INTO users VALUES (2, '"'"'Alan'"'"');
SELECT name FROM users WHERE id = 1;
SELECT * FROM users;
.exit
'
  local out3
  out3="$(printf '%s' "$sql_in" | timeout 20 $SRV repl 2>&1)"
  printf '%s\n' "$out3" | sed 's/^/    /'
  if printf '%s' "$out3" | grep -q 'Text("Ada")'; then
    ok "repl 多语句会话通过 (SELECT 命中 Ada)"
  else
    warn "repl 未命中预期结果, 请人工核对"
  fi
}

main() {
  log "sqlrustgo 第一周上机环境脚本"
  log "仓库根: $REPO_ROOT"
  check_toolchain
  build_bins || exit 1
  smoke_test || warn "冒烟测试未通过, 请手动检查子命令接口。"
  cat <<EOF

$(ok "全部完成。可用入口:")
  交互式 REPL (会话内持久, 推荐用于第一周 SQL 验证):
    cd $REPO_ROOT && ./$TARGET_DIR/sqlrustgo-mysql-server repl
  sqlrustgo-cli repl:
    cd $REPO_ROOT && ./$TARGET_DIR/sqlrustgo-cli repl
  单条 SQL:
    cd $REPO_ROOT && ./$TARGET_DIR/sqlrustgo-cli exec "SELECT 1"
  连接运行中的 server 执行 query:
    cd $REPO_ROOT && ./$TARGET_DIR/sqlrustgo-cli serve --port 3307 &
    ./$TARGET_DIR/sqlrustgo-cli cli -p 3307 "SELECT 1"
EOF
}

main "$@"
