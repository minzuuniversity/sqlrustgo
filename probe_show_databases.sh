#!/usr/bin/env bash
# #5009 缺陷 2 端到端探针 —— SHOW DATABASES 经真实 MySQL 协议必须列出用户库
#
# 为什么必须走 TCP（而不是 REPL）：
#   `repl` 子命令构造的是 MemoryExecutionEngine，直连 MemoryStorage。
#   生产路径 `serve` 构造的是 BoxStorageEngine（内含
#   WalStorage / MvccStorage / BinaryStorage 链）。
#   任何一层包装器不转发 list_databases()，就会静默继承 trait 默认值
#   （空列表）—— 而 REPL 与 executor 级测试对此结构性不可见。
#   这正是 #4974 的同一形状：组件是对的，组装没接上。
#
# 用法：probe_show_databases.sh <server-binary> [port]
# 期望：CREATE DATABASE shop 后，SHOW DATABASES 含 shop / default
# 修复前：只输出 default
set -uo pipefail

BIN="${1:?用法: probe_show_databases.sh <server-binary> [port]}"
PORT="${2:-3588}"
DIR=$(mktemp -d /private/tmp/probe5009.XXXXXX)

"$BIN" serve --host 127.0.0.1 --port "$PORT" --data-dir "$DIR" \
  >"$DIR/server.log" 2>&1 </dev/null &
SRV=$!
trap 'kill $SRV 2>/dev/null' EXIT

for _ in $(seq 1 60); do
  mysql -h 127.0.0.1 -P "$PORT" -u root -e "SELECT 1" >/dev/null 2>&1 && break
  sleep 1
done
if ! mysql -h 127.0.0.1 -P "$PORT" -u root -e "SELECT 1" >/dev/null 2>&1; then
  echo "PROBE server_start_fail log=$DIR/server.log"
  exit 1
fi

q() { mysql -h 127.0.0.1 -P "$PORT" --protocol=TCP -u root -N -B -e "$1" 2>&1; }

echo "PROBE before_create     dbs=[$(q 'SHOW DATABASES;' | tr '\n' ' ')]"

mysql -h 127.0.0.1 -P "$PORT" -u root -e "CREATE DATABASE shop;" >/dev/null 2>&1 \
  || echo "PROBE create_shop_failed"
mysql -h 127.0.0.1 -P "$PORT" -u root -e "CREATE DATABASE blog;" >/dev/null 2>&1 \
  || echo "PROBE create_blog_failed"

AFTER_CREATE=$(q 'SHOW DATABASES;')
echo "PROBE after_create      dbs=[$(echo "$AFTER_CREATE" | tr '\n' ' ')]"

mysql -h 127.0.0.1 -P "$PORT" -u root -e "DROP DATABASE blog;" >/dev/null 2>&1 \
  || echo "PROBE drop_blog_failed"
AFTER_DROP=$(q 'SHOW DATABASES;')
echo "PROBE after_drop        dbs=[$(echo "$AFTER_DROP" | tr '\n' ' ')]"

FAIL=0
grep -qx "shop"   <<<"$AFTER_CREATE" || { echo "FAIL: shop not listed after CREATE"; FAIL=1; }
grep -qx "blog"   <<<"$AFTER_CREATE" || { echo "FAIL: blog not listed after CREATE"; FAIL=1; }
grep -qx "default" <<<"$AFTER_CREATE" || { echo "FAIL: default not listed"; FAIL=1; }
grep -qx "shop"   <<<"$AFTER_DROP"   || { echo "FAIL: shop missing after DROP"; FAIL=1; }
grep -qx "blog"   <<<"$AFTER_DROP"   && { echo "FAIL: blog still listed after DROP"; FAIL=1; }

if [ "$FAIL" -eq 0 ]; then
  echo "VERDICT=CLEAN"
  exit 0
fi
echo "VERDICT=DEFECTIVE log=$DIR/server.log"
exit 1
