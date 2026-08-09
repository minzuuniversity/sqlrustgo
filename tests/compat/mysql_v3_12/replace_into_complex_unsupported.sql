# name: replace_into_complex_unsupported
# expect: PASS
CREATE TABLE t (id INT PRIMARY KEY);
CREATE TABLE t2 (id INT PRIMARY KEY);
INSERT INTO t2 VALUES (1);
REPLACE INTO t SELECT * FROM t2;
