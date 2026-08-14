# name: where_condition
# expect: PASS
CREATE TABLE t (a INT, b TEXT);
SELECT * FROM t WHERE a > 10;
