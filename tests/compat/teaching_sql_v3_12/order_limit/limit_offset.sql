# name: limit_offset
# expect: PASS
CREATE TABLE t (a INT);
INSERT INTO t VALUES (1), (2), (3), (4), (5);
SELECT * FROM t LIMIT 3 OFFSET 1;
