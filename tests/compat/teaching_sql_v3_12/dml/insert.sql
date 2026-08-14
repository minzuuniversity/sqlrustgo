# name: insert_basic
# expect: PASS
CREATE TABLE t (a INT, b TEXT);
INSERT INTO t VALUES (1, 'hello');
SELECT * FROM t;
