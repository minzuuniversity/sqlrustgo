# name: window_rank_partition_unsupported
# expect: DEFERRED: wire protocol truncation bug; see ISSUE #39XX
CREATE TABLE t (a INT, b INT, c INT);
INSERT INTO t VALUES (1, 1, 1);
SELECT RANK() OVER (PARTITION BY a, b ORDER BY c) FROM t;
