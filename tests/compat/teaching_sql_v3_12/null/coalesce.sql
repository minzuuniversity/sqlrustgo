# name: coalesce_function
# expect: PASS
CREATE TABLE t (a INT, b TEXT);
INSERT INTO t VALUES (1, NULL), (2, 'x');
SELECT a, COALESCE(b, 'default') FROM t;
