# name: median_unsupported
# expect: DEFERRED: returns NULL instead of error; see ISSUE #39XX
CREATE TABLE t (col INT);
INSERT INTO t VALUES (1);
SELECT MEDIAN(col) FROM t;
