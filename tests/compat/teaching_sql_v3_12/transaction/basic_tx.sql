# name: transaction_basic
# expect: PASS
CREATE TABLE t (a INT);
BEGIN;
INSERT INTO t VALUES (1);
COMMIT;
SELECT * FROM t;
