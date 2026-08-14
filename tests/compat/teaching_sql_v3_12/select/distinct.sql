# name: select_distinct
# expect: PASS
CREATE TABLE t (a INT, b TEXT);
INSERT INTO t VALUES (1, 'x'), (1, 'x'), (2, 'y');
SELECT DISTINCT a FROM t;
