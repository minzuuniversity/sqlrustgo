CREATE TABLE t (id INTEGER, name TEXT);
INSERT INTO t VALUES (1, 'alice');
SELECT nonexistent FROM t;
INSERT INTO t VALUES (2, 'bob');
SELECT id, name FROM t;