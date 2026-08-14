# name: update_basic
# expect: PASS
CREATE TABLE t (a INT, b TEXT);
INSERT INTO t VALUES (1, 'old');
UPDATE t SET b = 'new' WHERE a = 1;
SELECT b FROM t;
