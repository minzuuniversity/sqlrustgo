# name: var_pop_unsupported
# expect: PASS
CREATE TABLE t (col INT);
INSERT INTO t VALUES (1);
SELECT VAR_POP(col) FROM t;
