#!/usr/bin/env python3
import time, pymysql
LOG = "/tmp/a04-sqlrustgo-server.log"
SETUP = ["DROP TABLE IF EXISTS hvx",
         "CREATE TABLE hvx (id INT, dept INT)",
         "INSERT INTO hvx VALUES (1,1),(2,1),(3,2),(4,3),(5,3),(6,4)"]

def panic_count():
    return sum(1 for line in open(LOG) if "panicked at" in line)

def run(sql):
    conn = pymysql.connect(host="127.0.0.1", port=3309, user="root")
    cur = conn.cursor()
    for s in SETUP:
        cur.execute(s)
    cur.close()
    cur = conn.cursor()
    p0 = panic_count()
    try:
        cur.execute(sql)
        res = ("OK", tuple(cur.fetchall()) if cur.description else None)
    except Exception as e:
        res = ("ERR", e.args[0], str(e)[:70])
    conn.close()
    time.sleep(1.0)
    p1 = panic_count()
    return res, (p1 - p0)

for label, sql in [
    ("A 1col HAVING COUNT=2",  "SELECT dept FROM hvx GROUP BY dept HAVING COUNT(*) = 2"),
    ("B 1col HAVING COUNT>1",  "SELECT dept FROM hvx GROUP BY dept HAVING COUNT(*) > 1"),
    ("C 1col HAVING COUNT<3",  "SELECT dept FROM hvx GROUP BY dept HAVING COUNT(*) < 3"),
    ("D 2col HAVING COUNT=2",  "SELECT dept, COUNT(*) AS c FROM hvx GROUP BY dept HAVING COUNT(*) = 2"),
    ("E 1col no HAVING",       "SELECT dept FROM hvx GROUP BY dept"),
    ("F 1col HAVING col=2",    "SELECT dept FROM hvx GROUP BY dept HAVING dept = 2"),
    ("G 1col HAVING SUM=2",    "SELECT dept FROM hvx GROUP BY dept HAVING SUM(dept) = 4"),
]:
    res, pd = run(sql)
    print("%-24s panic_delta=%d  %s" % (label, pd, res))
