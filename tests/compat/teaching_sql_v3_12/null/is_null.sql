# name: is_null_check
# expect: PASS
CREATE TABLE t (a INT, b TEXT);
INSERT INTO t VALUES (1, NULL), (2, 'x'), (3, NULL);
SELECT * FROM t WHERE b IS NULL;
