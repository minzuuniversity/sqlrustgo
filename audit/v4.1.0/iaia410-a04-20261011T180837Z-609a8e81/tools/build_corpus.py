#!/usr/bin/env python3
"""Build the A04 full differential corpus (all SSOT coverage domains).
Each case is self-contained (DROP+CREATE+INSERT in setup) so subject and
reference start from equivalent state. Emits corpus_full.json + its sha256.
"""
import json, hashlib, sys

C = []
def case(id, domain, query, setup=(), order=False, **kw):
    c = {"id": id, "domain": domain, "order_sensitive": order}
    c.update(kw)
    if setup:
        c["setup"] = list(setup)
    c["query"] = query
    C.append(c)

# ---------------- coltype (field-type metadata) ----------------
case("A04-TYPE-001", "coltype", "SELECT 1 AS i, 'x' AS s, 3.5 AS d, NULL AS n, 1.0 AS f")
case("A04-TYPE-002", "coltype", "SELECT i FROM t_int",
     setup=["DROP TABLE IF EXISTS t_int", "CREATE TABLE t_int (i INT)",
            "INSERT INTO t_int VALUES (42)"])
case("A04-TYPE-003", "coltype", "SELECT COUNT(*) AS c, SUM(i) AS s, MAX(i) AS m, MIN(i) AS n FROM t_int3",
     setup=["DROP TABLE IF EXISTS t_int3", "CREATE TABLE t_int3 (i INT)",
            "INSERT INTO t_int3 VALUES (1),(2),(3)"])
case("A04-TYPE-004", "coltype", "SELECT b FROM t_big",
     setup=["DROP TABLE IF EXISTS t_big", "CREATE TABLE t_big (b BIGINT)",
            "INSERT INTO t_big VALUES (9999999999)"])
case("A04-TYPE-005", "coltype", "SELECT d FROM t_dbl",
     setup=["DROP TABLE IF EXISTS t_dbl", "CREATE TABLE t_dbl (d DOUBLE)",
            "INSERT INTO t_dbl VALUES (3.25)"])
case("A04-TYPE-006", "coltype", "SELECT d FROM t_dec",
     setup=["DROP TABLE IF EXISTS t_dec", "CREATE TABLE t_dec (d DECIMAL(10,2))",
            "INSERT INTO t_dec VALUES (12.34)"])
case("A04-TYPE-007", "coltype", "SELECT dt FROM t_date",
     setup=["DROP TABLE IF EXISTS t_date", "CREATE TABLE t_date (dt DATE)",
            "INSERT INTO t_date VALUES ('2024-01-15')"])
case("A04-TYPE-008", "coltype", "SELECT id, s, d, dt FROM t_multi ORDER BY id",
     setup=["DROP TABLE IF EXISTS t_multi",
            "CREATE TABLE t_multi (id INT, s VARCHAR(20), d DECIMAL(10,2), dt DATE)",
            "INSERT INTO t_multi VALUES (1,'a',1.50,'2024-01-01')"])

# ---------------- join ----------------
case("A04-JOIN-001", "join", "SELECT j_a.id, j_b.name FROM j_a JOIN j_b ON j_a.bid=j_b.id ORDER BY j_a.id",
     setup=["DROP TABLE IF EXISTS j_a", "CREATE TABLE j_a (id INT, bid INT)",
            "INSERT INTO j_a VALUES (1,10),(2,20),(3,30)",
            "DROP TABLE IF EXISTS j_b", "CREATE TABLE j_b (id INT, name VARCHAR(20))",
            "INSERT INTO j_b VALUES (10,'x'),(20,'y')"])
case("A04-JOIN-002", "join", "SELECT j_a2.id, j_b2.name FROM j_a2 LEFT JOIN j_b2 ON j_a2.bid=j_b2.id ORDER BY j_a2.id",
     setup=["DROP TABLE IF EXISTS j_a2", "CREATE TABLE j_a2 (id INT, bid INT)",
            "INSERT INTO j_a2 VALUES (1,10),(2,99)",
            "DROP TABLE IF EXISTS j_b2", "CREATE TABLE j_b2 (id INT, name VARCHAR(20))",
            "INSERT INTO j_b2 VALUES (10,'x')"])
case("A04-JOIN-003", "join", "SELECT j_c.id, j_d.name FROM j_c, j_d WHERE j_c.bid=j_d.id ORDER BY j_c.id",
     setup=["DROP TABLE IF EXISTS j_c", "CREATE TABLE j_c (id INT, bid INT)",
            "INSERT INTO j_c VALUES (1,10),(2,20)",
            "DROP TABLE IF EXISTS j_d", "CREATE TABLE j_d (id INT, name VARCHAR(20))",
            "INSERT INTO j_d VALUES (10,'x'),(20,'y')"])
case("A04-JOIN-004", "join", "SELECT a.id, b.id FROM j_self a JOIN j_self b ON a.id < b.id ORDER BY a.id, b.id",
     setup=["DROP TABLE IF EXISTS j_self", "CREATE TABLE j_self (id INT)",
            "INSERT INTO j_self VALUES (1),(2),(3)"])
case("A04-JOIN-005", "join", "SELECT a.id, b.name, c.tag FROM j_3a a JOIN j_3b b ON a.bid=b.id JOIN j_3c c ON b.id=c.id ORDER BY a.id",
     setup=["DROP TABLE IF EXISTS j_3a", "CREATE TABLE j_3a (id INT, bid INT)",
            "INSERT INTO j_3a VALUES (1,10),(2,20)",
            "DROP TABLE IF EXISTS j_3b", "CREATE TABLE j_3b (id INT, name VARCHAR(20))",
            "INSERT INTO j_3b VALUES (10,'x'),(20,'y')",
            "DROP TABLE IF EXISTS j_3c", "CREATE TABLE j_3c (id INT, tag VARCHAR(20))",
            "INSERT INTO j_3c VALUES (10,'T'),(20,'U')"])

# ---------------- aggregate ----------------
case("A04-AGG-001", "aggregate", "SELECT dept, COUNT(*) AS c FROM agg_e GROUP BY dept ORDER BY dept",
     setup=["DROP TABLE IF EXISTS agg_e", "CREATE TABLE agg_e (id INT, dept INT)",
            "INSERT INTO agg_e VALUES (1,1),(2,1),(3,2),(4,2),(5,3)"])
case("A04-AGG-002", "aggregate", "SELECT dept, AVG(v) AS a, MIN(v) AS lo, MAX(v) AS hi FROM agg_e2 GROUP BY dept ORDER BY dept",
     setup=["DROP TABLE IF EXISTS agg_e2", "CREATE TABLE agg_e2 (id INT, dept INT, v INT)",
            "INSERT INTO agg_e2 VALUES (1,1,10),(2,1,20),(3,2,5),(4,2,15)"])
case("A04-AGG-003", "aggregate", "SELECT COUNT(*) AS c, SUM(v) AS s FROM agg_e3",
     setup=["DROP TABLE IF EXISTS agg_e3", "CREATE TABLE agg_e3 (id INT, v INT)",
            "INSERT INTO agg_e3 VALUES (1,10),(2,20),(3,30)"])
case("A04-AGG-004", "aggregate", "SELECT a, b, COUNT(*) AS c FROM agg_e4 GROUP BY a, b ORDER BY a, b",
     setup=["DROP TABLE IF EXISTS agg_e4", "CREATE TABLE agg_e4 (id INT, a INT, b INT)",
            "INSERT INTO agg_e4 VALUES (1,1,1),(2,1,1),(3,1,2),(4,2,1),(5,2,2),(6,2,2)"])
case("A04-AGG-005", "aggregate", "SELECT COUNT(DISTINCT dept) AS cd FROM agg_e5",
     setup=["DROP TABLE IF EXISTS agg_e5", "CREATE TABLE agg_e5 (id INT, dept INT)",
            "INSERT INTO agg_e5 VALUES (1,1),(2,1),(3,2),(4,2),(5,3)"])

# ---------------- having ----------------
case("A04-HAVING-001", "having", "SELECT dept, COUNT(*) AS c FROM hv_e GROUP BY dept HAVING COUNT(*) > 1 ORDER BY dept",
     setup=["DROP TABLE IF EXISTS hv_e", "CREATE TABLE hv_e (id INT, dept INT)",
            "INSERT INTO hv_e VALUES (1,1),(2,1),(3,2),(4,2),(5,3)"])
case("A04-HAVING-002", "having", "SELECT dept, SUM(v) AS s FROM hv_e2 GROUP BY dept HAVING SUM(v) >= 15 ORDER BY dept",
     setup=["DROP TABLE IF EXISTS hv_e2", "CREATE TABLE hv_e2 (id INT, dept INT, v INT)",
            "INSERT INTO hv_e2 VALUES (1,1,10),(2,1,20),(3,2,5),(4,2,15)"])
case("A04-HAVING-003", "having", "SELECT dept FROM hv_e3 GROUP BY dept HAVING COUNT(*) = 2 ORDER BY dept",
     setup=["DROP TABLE IF EXISTS hv_e3", "CREATE TABLE hv_e3 (id INT, dept INT)",
            "INSERT INTO hv_e3 VALUES (1,1),(2,1),(3,2),(4,3),(5,3),(6,4)"])

# ---------------- subquery ----------------
case("A04-SUBQ-001", "subquery", "SELECT id FROM sq_t WHERE v > (SELECT AVG(v) FROM sq_t) ORDER BY id",
     setup=["DROP TABLE IF EXISTS sq_t", "CREATE TABLE sq_t (id INT, v INT)",
            "INSERT INTO sq_t VALUES (1,10),(2,20),(3,30),(4,40)"])
case("A04-SUBQ-002", "subquery", "SELECT id FROM sq_t2 WHERE dept IN (SELECT dept FROM sq_t2 WHERE dept IN (1,3)) ORDER BY id",
     setup=["DROP TABLE IF EXISTS sq_t2", "CREATE TABLE sq_t2 (id INT, dept INT)",
            "INSERT INTO sq_t2 VALUES (1,1),(2,2),(3,3)"])
case("A04-SUBQ-003", "subquery", "SELECT id FROM sq_t3 WHERE EXISTS (SELECT 1 FROM sq_t4 WHERE sq_t4.id = sq_t3.id) ORDER BY id",
     setup=["DROP TABLE IF EXISTS sq_t3", "CREATE TABLE sq_t3 (id INT)",
            "INSERT INTO sq_t3 VALUES (1),(2),(3)",
            "DROP TABLE IF EXISTS sq_t4", "CREATE TABLE sq_t4 (id INT)",
            "INSERT INTO sq_t4 VALUES (1),(3)"])
case("A04-SUBQ-004", "subquery", "SELECT id, (SELECT MAX(v) FROM sq_t5 WHERE sq_t5.id = sq_t6.id) AS mv FROM sq_t6 ORDER BY id",
     setup=["DROP TABLE IF EXISTS sq_t5", "CREATE TABLE sq_t5 (id INT, v INT)",
            "INSERT INTO sq_t5 VALUES (1,10),(2,20),(3,30)",
            "DROP TABLE IF EXISTS sq_t6", "CREATE TABLE sq_t6 (id INT)",
            "INSERT INTO sq_t6 VALUES (1),(2),(3)"])

# ---------------- null / 3-valued logic ----------------
case("A04-NULL-001", "null", "SELECT id FROM nl_t WHERE dept = NULL",
     setup=["DROP TABLE IF EXISTS nl_t", "CREATE TABLE nl_t (id INT, dept INT)",
            "INSERT INTO nl_t VALUES (1,NULL),(2,1),(3,2)"])
case("A04-NULL-002", "null", "SELECT id FROM nl_t2 WHERE dept IS NULL ORDER BY id",
     setup=["DROP TABLE IF EXISTS nl_t2", "CREATE TABLE nl_t2 (id INT, dept INT)",
            "INSERT INTO nl_t2 VALUES (1,NULL),(2,1),(3,2)"])
case("A04-NULL-003", "null", "SELECT id FROM nl_t3 WHERE dept <> NULL ORDER BY id",
     setup=["DROP TABLE IF EXISTS nl_t3", "CREATE TABLE nl_t3 (id INT, dept INT)",
            "INSERT INTO nl_t3 VALUES (1,NULL),(2,1),(3,2)"])
case("A04-NULL-004", "null", "SELECT id, v + NULL AS r FROM nl_t4 ORDER BY id",
     setup=["DROP TABLE IF EXISTS nl_t4", "CREATE TABLE nl_t4 (id INT, v INT)",
            "INSERT INTO nl_t4 VALUES (1,10),(2,20)"])
case("A04-NULL-005", "null", "SELECT id, COALESCE(dept, -1) AS d FROM nl_t5 ORDER BY id",
     setup=["DROP TABLE IF EXISTS nl_t5", "CREATE TABLE nl_t5 (id INT, dept INT)",
            "INSERT INTO nl_t5 VALUES (1,NULL),(2,5)"])
case("A04-NULL-006", "null", "SELECT id FROM nl_t6 WHERE dept IS NOT NULL ORDER BY id",
     setup=["DROP TABLE IF EXISTS nl_t6", "CREATE TABLE nl_t6 (id INT, dept INT)",
            "INSERT INTO nl_t6 VALUES (1,NULL),(2,1),(3,2)"])

# ---------------- order / limit ----------------
case("A04-ORDER-001", "order_limit", "SELECT id FROM od_t ORDER BY id DESC LIMIT 3",
     setup=["DROP TABLE IF EXISTS od_t", "CREATE TABLE od_t (id INT)",
            "INSERT INTO od_t VALUES (5),(1),(3),(2),(4)"])
case("A04-ORDER-002", "order_limit", "SELECT id FROM od_t2 ORDER BY id LIMIT 1, 3",
     setup=["DROP TABLE IF EXISTS od_t2", "CREATE TABLE od_t2 (id INT)",
            "INSERT INTO od_t2 VALUES (1),(2),(3),(4),(5)"])
case("A04-ORDER-003", "order_limit", "SELECT id, dept FROM od_t3 ORDER BY dept, id",
     setup=["DROP TABLE IF EXISTS od_t3", "CREATE TABLE od_t3 (id INT, dept INT)",
            "INSERT INTO od_t3 VALUES (1,1),(2,2),(3,1),(4,2)"])
case("A04-ORDER-004", "order_limit", "SELECT id FROM od_t4 LIMIT 2",
     setup=["DROP TABLE IF EXISTS od_t4", "CREATE TABLE od_t4 (id INT)",
            "INSERT INTO od_t4 VALUES (7),(2),(5)"])

# ---------------- type conversion ----------------
case("A04-CAST-001", "typeconv", "SELECT i + 1 AS p, i * 2 AS d2 FROM tc_i ORDER BY i",
     setup=["DROP TABLE IF EXISTS tc_i", "CREATE TABLE tc_i (i INT)",
            "INSERT INTO tc_i VALUES (1),(2),(3)"])
case("A04-CAST-002", "typeconv", "SELECT id FROM tc_s WHERE s = '42' ORDER BY id",
     setup=["DROP TABLE IF EXISTS tc_s", "CREATE TABLE tc_s (id INT, s VARCHAR(20))",
            "INSERT INTO tc_s VALUES (1,'42'),(2,'42x'),(3,' 42 ')"])
case("A04-CAST-003", "typeconv", "SELECT id, CAST(i AS CHAR) AS c FROM tc_c ORDER BY id",
     setup=["DROP TABLE IF EXISTS tc_c", "CREATE TABLE tc_c (id INT, i INT)",
            "INSERT INTO tc_c VALUES (1,10),(2,20)"])
case("A04-CAST-004", "typeconv", "SELECT id, CONCAT(s, '-') AS g FROM tc_g ORDER BY id",
     setup=["DROP TABLE IF EXISTS tc_g", "CREATE TABLE tc_g (id INT, s VARCHAR(20))",
            "INSERT INTO tc_g VALUES (1,'a'),(2,'b')"])
case("A04-CAST-005", "typeconv", "SELECT id FROM tc_n WHERE i = '5' ORDER BY id",
     setup=["DROP TABLE IF EXISTS tc_n", "CREATE TABLE tc_n (id INT, i INT)",
            "INSERT INTO tc_n VALUES (1,5),(2,50),(3,500)"])

# ---------------- error category ----------------
case("A04-ERR-001", "error", "SELECT badcol FROM er_t",
     setup=["DROP TABLE IF EXISTS er_t", "CREATE TABLE er_t (id INT)",
            "INSERT INTO er_t VALUES (1)"])
case("A04-ERR-002", "error", "SELECT * FROM definitely_not_a_table")
case("A04-ERR-003", "error", "SELECT id FROM er_t WHERE",
     setup=["DROP TABLE IF EXISTS er_t3", "CREATE TABLE er_t3 (id INT)",
            "INSERT INTO er_t3 VALUES (1)"])
case("A04-ERR-004", "error", "SELECT id FROM er_t4 WHERE id = (SELECT 1 FROM nonexist_tbl)",
     setup=["DROP TABLE IF EXISTS er_t4", "CREATE TABLE er_t4 (id INT)",
            "INSERT INTO er_t4 VALUES (1)"])

# ---------------- column names ----------------
case("A04-COL-001", "colname", "SELECT id AS x, s AS y FROM cn_t ORDER BY id",
     setup=["DROP TABLE IF EXISTS cn_t", "CREATE TABLE cn_t (id INT, s VARCHAR(20))",
            "INSERT INTO cn_t VALUES (1,'a')"])
case("A04-COL-002", "colname", "SELECT * FROM cn_t2 ORDER BY id",
     setup=["DROP TABLE IF EXISTS cn_t2", "CREATE TABLE cn_t2 (id INT, s VARCHAR(20))",
            "INSERT INTO cn_t2 VALUES (1,'a')"])
case("A04-COL-003", "colname", "SELECT a + b FROM cn_t3",
     setup=["DROP TABLE IF EXISTS cn_t3", "CREATE TABLE cn_t3 (a INT, b INT)",
            "INSERT INTO cn_t3 VALUES (1,2)"])
case("A04-COL-004", "colname", "SELECT id, id AS id2 FROM cn_t4 ORDER BY id",
     setup=["DROP TABLE IF EXISTS cn_t4", "CREATE TABLE cn_t4 (id INT)",
            "INSERT INTO cn_t4 VALUES (1)"])

# ---------------- DDL ----------------
case("A04-DDL-001", "ddl", "SELECT id, s, d FROM dd_t ORDER BY id",
     setup=["DROP TABLE IF EXISTS dd_t",
            "CREATE TABLE dd_t (id INT PRIMARY KEY, s VARCHAR(20), d DECIMAL(10,2))",
            "INSERT INTO dd_t VALUES (1,'a',1.50)"])
case("A04-DDL-002", "ddl", "SELECT COLUMN_NAME, DATA_TYPE FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_NAME='dd_t2' ORDER BY ORDINAL_POSITION",
     setup=["DROP TABLE IF EXISTS dd_t2",
            "CREATE TABLE dd_t2 (id INT, s VARCHAR(20), d DECIMAL(10,2))"])
case("A04-DDL-003", "ddl", "SELECT COUNT(*) AS c FROM dd_t3",
     setup=["DROP TABLE IF EXISTS dd_t3",
            "CREATE TABLE dd_t3 (id INT NOT NULL, s VARCHAR(20) DEFAULT 'x')",
            "INSERT INTO dd_t3 (id) VALUES (1)"])

# ---------------- constraint ----------------
case("A04-CONSTRAINT-001", "constraint", "INSERT INTO ct_pk VALUES (1,'dup')",
     setup=["DROP TABLE IF EXISTS ct_pk",
            "CREATE TABLE ct_pk (id INT PRIMARY KEY, s VARCHAR(20))",
            "INSERT INTO ct_pk VALUES (1,'a')"])
case("A04-CONSTRAINT-002", "constraint", "INSERT INTO ct_uq VALUES (1) ON DUPLICATE KEY UPDATE id=1",
     setup=["DROP TABLE IF EXISTS ct_uq",
            "CREATE TABLE ct_uq (id INT UNIQUE)"])
case("A04-CONSTRAINT-003", "constraint", "SELECT id FROM ct_nn ORDER BY id",
     setup=["DROP TABLE IF EXISTS ct_nn",
            "CREATE TABLE ct_nn (id INT NOT NULL)",
            "INSERT INTO ct_nn VALUES (1),(2)"])

raw = json.dumps({"seed": 410, "cases": C}, sort_keys=True, ensure_ascii=False).encode()
corpus_sha = hashlib.sha256(raw).hexdigest()
with open("/tmp/a04/corpus_full.json", "w") as f:
    json.dump({"seed": 410, "corpus_sha256": corpus_sha, "cases": C}, f, indent=2, ensure_ascii=False)

print("cases:", len(C))
print("corpus_sha256:", corpus_sha)
domains = sorted(set(c["domain"] for c in C))
print("domains:", domains)
