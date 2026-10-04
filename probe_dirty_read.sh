#!/usr/bin/env bash
# #4974 端到端脏读探针 —— 真实 MySQL 协议 + 两个并发连接
#
# 目的：证明「SELECT 主路径走 storage.scan() 而非 scan_in()」在系统层面
# 确实会让未提交的写对其他连接可见。组件级测试（直接调 VersionedTable /
# MvccStorage）8/8 全绿，抓不到这个 —— 因为组件是对的，组装没接上。
#
# 协议：
#   连接 A: BEGIN -> INSERT -> (保持未提交 3 秒) -> COMMIT
#   连接 B: 在 A 未提交期间 SELECT COUNT(*)
#
# 期望：B 读到 0 行；A 提交后 B 读到 1 行。
# 实际（修复前）：B 在 A 未提交期间就读到 1 行 —— 脏读。
set -uo pipefail

BIN="${1:?用法: probe_dirty_read.sh <server-binary>}"
PORT="${2:-3577}"
EXTRA_ARGS="${3:-}"
DIR=$(mktemp -d /private/tmp/probe4974.XXXXXX)
DB=probe

"$BIN" serve --host 127.0.0.1 --port "$PORT" --data-dir "$DIR" $EXTRA_ARGS \
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

mysql -h 127.0.0.1 -P "$PORT" -u root -e "CREATE DATABASE IF NOT EXISTS $DB;" >/dev/null
mysql -h 127.0.0.1 -P "$PORT" -u root "$DB" -e \
  "CREATE TABLE t (id INT PRIMARY KEY, v CHAR(10));" >/dev/null

q() { mysql -h 127.0.0.1 -P "$PORT" --protocol=TCP -u root "$DB" -N -B -e "$1" 2>&1; }

echo "PROBE before_any_tx      count=$(q 'SELECT COUNT(*) FROM t;')"

# 连接 A：BEGIN + INSERT，然后保持管道不关（事务因此停在未提交状态）
{ echo "BEGIN;"; echo "INSERT INTO t VALUES (1,'x');"; sleep 3; echo "COMMIT;"; sleep 1; } \
  | mysql -h 127.0.0.1 -P "$PORT" --protocol=TCP -u root "$DB" >"$DIR/connA.log" 2>&1 &
APID=$!
sleep 1.2   # 让 A 完成 INSERT 并停在未提交状态

DIRTY=$(q 'SELECT COUNT(*) FROM t;')
echo "PROBE while_A_uncommitted count=$DIRTY  (期望 0；非 0 即脏读)"

wait $APID 2>/dev/null
AFTER=$(q 'SELECT COUNT(*) FROM t;')
echo "PROBE after_A_commit     count=$AFTER  (期望 1)"

AERR=$(grep -ci "error" "$DIR/connA.log" 2>/dev/null || echo 0)
echo "PROBE connA_errors=$AERR"

if [ "$DIRTY" != "0" ]; then
  echo "PROBE VERDICT=DIRTY_READ_REPRODUCED"
  exit 3
fi
if [ "$AFTER" != "1" ]; then
  echo "PROBE VERDICT=LOST_WRITE"
  exit 4
fi
echo "PROBE VERDICT=CLEAN"
exit 0
