# name: select_alias
# expect: PASS
CREATE TABLE t (a INT, b TEXT);
SELECT a AS id, b AS name FROM t;
