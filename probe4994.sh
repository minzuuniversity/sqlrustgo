#!/usr/bin/env bash
# #4994 复现脚本 —— 高并发 + DELETE 下「所有语句成功但数据全丢」
#
# 用法: probe4994.sh <server-binary> <port> <workers> <rounds>
#
# 刻意不吞 stderr —— 上一版脚本用 2>/dev/null 让「全失败」伪装成「无异常」。
set -uo pipefail

BIN="${1:?binary}"; PORT="${2:-3700}"; W="${3:-8}"; R="${4:-12}"
DIR=$(mktemp -d /private/tmp/p4994.XXXXXX)
DB=p4994

"$BIN" serve --host 127.0.0.1 --port "$PORT" --data-dir "$DIR" \
  >"$DIR/server.log" 2>&1 </dev/null &
SRV=$!
trap 'kill $SRV 2>/dev/null' EXIT
for _ in $(seq 1 60); do
  mysql -h 127.0.0.1 -P "$PORT" -u root -e "SELECT 1" >/dev/null 2>&1 && break
  sleep 1
done

q() { mysql -h 127.0.0.1 -P "$PORT" --protocol=TCP -u root "$DB" -N -B -e "$1" 2>&1; }
mysql -h 127.0.0.1 -P "$PORT" -u root -e "CREATE DATABASE IF NOT EXISTS $DB;" >/dev/null 2>&1
q "CREATE TABLE t (id INT PRIMARY KEY, v INT);" >/dev/null
q "INSERT INTO t VALUES (1,0);" >/dev/null          # seed 行

worker() {
  local w="$1"
  local ok=0 err=0
  for i in $(seq 1 "$R"); do
    local id=$((w*1000+i))
    # ODKU 插入（每 worker 写自己的 id 空间）
    o=$(q "INSERT INTO t VALUES ($id, $w) ON DUPLICATE KEY UPDATE v=$w;")
    if [ -z "$o" ]; then ok=$((ok+1)); else err=$((err+1)); fi
    # UPDATE
    o=$(q "UPDATE t SET v=v+1 WHERE id=$id;")
    if [ -z "$o" ]; then ok=$((ok+1)); else err=$((err+1)); fi
    # SELECT
    o=$(q "SELECT v FROM t WHERE id=$id;")
    if [ -n "$o" ] || [ $? -eq 0 ]; then ok=$((ok+1)); else err=$((err+1)); fi
    # DELETE（仅 i > 8）
    if [ "$i" -gt 8 ]; then
      o=$(q "DELETE FROM t WHERE id=$id;")
      if [ -z "$o" ]; then ok=$((ok+1)); else err=$((err+1)); fi
    fi
  done
  # 每 worker 末尾 4 次显式事务
  for _ in 1 2 3 4; do
    o=$(q "BEGIN; UPDATE t SET v=v WHERE id=$w*1000+1; COMMIT;")
    if [ -z "$o" ]; then ok=$((ok+1)); else err=$((err+1)); fi
  done
  echo "worker$w ok=$ok err=$err"
}

export -f worker q
export PORT DB R

echo "=== #4994 workers=$W rounds=$R (DELETE 分支: i>8) ==="
TMPW=$(mktemp -d /private/tmp/p4994w.XXXXXX)
for w in $(seq 1 "$W"); do
  ( worker "$w" > "$TMPW/w$w.txt" 2>&1 ) &
done
wait
cat "$TMPW"/w*.txt

ROWS=$(q "SELECT COUNT(*) FROM t;")
EXPECT=$(( 1 + W*8 ))
ALIVE=$(pgrep -f "port $PORT" >/dev/null && echo yes || echo no)
SQLERR=$(grep -ci "SQL error" "$DIR/server.log" 2>/dev/null || echo 0)
PANIC=$(grep -ciE "panic|deadlock" "$DIR/server.log" 2>/dev/null || echo 0)

echo "SERVER_ALIVE=$ALIVE"
echo "PANIC_OR_DEADLOCK=$PANIC"
echo "SQL_ERRORS=$SQLERR"
echo "ROWS=$ROWS  (预期 $EXPECT)"
if [ "$ROWS" = "$EXPECT" ]; then echo "VERDICT=PASS"; else echo "VERDICT=ROWS_LOST"; fi
