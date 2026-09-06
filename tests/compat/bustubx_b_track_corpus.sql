-- BustubX-EDU B-Track Compatibility Corpus
--
-- This file contains SQL sequences from BustubX-EDU differential_test.py
-- that exposed bugs in SQLRustGo. Each section represents a test case.
--
-- Format:
-- -- TEST: <test_id> - <description>
-- <SQL statements separated by semicolons>
-- -- EXPECTED: <what should happen>
--
-- These tests should be run as stateful sequences, not isolated queries.

-- =============================================================================
-- P3-MATH-001: MOD returns Float instead of Integer
-- =============================================================================
-- TEST: P3-MATH-001 - MOD integer return type
CREATE TABLE t_math_mod (a INT, b INT);
INSERT INTO t_math_mod VALUES (10, 3);
SELECT MOD(a, b) FROM t_math_mod;
-- EXPECTED: Returns Integer(1), not Float(1.0)
-- STATUS: FIXED in PR #4833

-- =============================================================================
-- P3-WIN-002: NTILE(4) over 7 rows distribution error
-- =============================================================================
-- TEST: P3-WIN-002 - NTILE distribution
CREATE TABLE t_ntile (id INT);
INSERT INTO t_ntile VALUES (1),(2),(3),(4),(5),(6),(7);
SELECT id, NTILE(4) OVER (ORDER BY id) AS bucket FROM t_ntile ORDER BY id;
-- EXPECTED: 1,1 / 2,1 / 3,2 / 4,2 / 5,3 / 6,3 / 7,4
-- STATUS: NOT IMPLEMENTED (NTILE missing from WindowFunction enum)

-- =============================================================================
-- P3-DDL-001: DROP INDEX reports "does not exist"
-- =============================================================================
-- TEST: P3-DDL-001 - DROP INDEX after CREATE INDEX
CREATE TABLE t_drop_idx (id INT, name TEXT);
INSERT INTO t_drop_idx VALUES (1, 'a'), (2, 'b'), (3, 'c');
CREATE INDEX idx_t_drop_name ON t_drop_idx(name);
DROP INDEX idx_t_drop_name;
-- EXPECTED: DROP INDEX succeeds
-- STATUS: CANNOT REPRODUCE (existing tests pass)

-- =============================================================================
-- P3-HINT-001: INDEXED BY hint ignored
-- =============================================================================
-- TEST: P3-HINT-001 - INDEXED BY hint
CREATE TABLE t_hint (id INT, name TEXT);
INSERT INTO t_hint VALUES (1, 'a'), (2, 'b');
CREATE INDEX idx_hint_name ON t_hint(name);
SELECT * FROM t_hint INDEXED BY idx_hint_name WHERE name = 'a';
-- EXPECTED: Uses index, returns row with name='a'
-- STATUS: NOT IMPLEMENTED (INDEXED BY not in parser/executor)

-- =============================================================================
-- P3-NULL-001: NULL handling in comparisons
-- =============================================================================
-- TEST: P3-NULL-001 - NULL in WHERE clause
CREATE TABLE t_null_cmp (id INT, val INT);
INSERT INTO t_null_cmp VALUES (1, NULL), (2, 10), (3, NULL);
SELECT * FROM t_null_cmp WHERE val = NULL;
SELECT * FROM t_null_cmp WHERE val IS NULL;
SELECT * FROM t_null_cmp WHERE val <> 10;
-- EXPECTED:
-- First query: 0 rows (val = NULL returns empty)
-- Second query: 2 rows (id 1 and 3)
-- Third query: 0 rows (NULL <> 10 returns empty)
-- STATUS: NEEDS VERIFICATION

-- =============================================================================
-- P3-AGG-001: SUM of empty result
-- =============================================================================
-- TEST: P3-AGG-001 - SUM empty
CREATE TABLE t_sum_empty (val INT);
SELECT SUM(val) FROM t_sum_empty;
-- EXPECTED: NULL (not 0)
-- STATUS: NEEDS VERIFICATION

-- =============================================================================
-- P3-AGG-002: COUNT vs SUM semantics
-- =============================================================================
-- TEST: P3-AGG-002 - COUNT vs SUM
CREATE TABLE t_count_sum (val INT);
INSERT INTO t_count_sum VALUES (1), (2), (3), (NULL);
SELECT COUNT(val), SUM(val) FROM t_count_sum;
-- EXPECTED: COUNT=3, SUM=6 (NULL excluded from SUM)
-- STATUS: NEEDS VERIFICATION

-- =============================================================================
-- P3-JOIN-001: 3-table JOIN
-- =============================================================================
-- TEST: P3-JOIN-001 - 3-table join
CREATE TABLE t_j1 (id INT, j1_id INT);
CREATE TABLE t_j2 (id INT, j2_id INT);
CREATE TABLE t_j3 (id INT, j3_id INT);
INSERT INTO t_j1 VALUES (1, 10), (2, 20);
INSERT INTO t_j2 VALUES (10, 100), (20, 200);
INSERT INTO t_j3 VALUES (100, 1000), (200, 2000);
SELECT * FROM t_j1
  JOIN t_j2 ON t_j1.j1_id = t_j2.id
  JOIN t_j3 ON t_j2.j2_id = t_j3.id;
-- EXPECTED: 2 rows with joined data
-- STATUS: NEEDS VERIFICATION

-- =============================================================================
-- P3-SUB-001: EXISTS subquery
-- =============================================================================
-- TEST: P3-SUB-001 - EXISTS
CREATE TABLE t_sub1 (id INT, name TEXT);
CREATE TABLE t_sub2 (id INT, sub1_id INT);
INSERT INTO t_sub1 VALUES (1, 'a'), (2, 'b'), (3, 'c');
INSERT INTO t_sub2 VALUES (10, 1), (20, 2);
SELECT * FROM t_sub1 WHERE EXISTS (
  SELECT 1 FROM t_sub2 WHERE t_sub2.sub1_id = t_sub1.id
);
-- EXPECTED: 2 rows (id 1 and 2)
-- STATUS: NEEDS VERIFICATION

-- =============================================================================
-- P3-SET-001: UNION ALL
-- =============================================================================
-- TEST: P3-SET-001 - UNION ALL
CREATE TABLE t_set1 (id INT);
CREATE TABLE t_set2 (id INT);
INSERT INTO t_set1 VALUES (1), (2);
INSERT INTO t_set2 VALUES (2), (3);
SELECT * FROM t_set1 UNION ALL SELECT * FROM t_set2 ORDER BY id;
-- EXPECTED: 1, 2, 2, 3
-- STATUS: NEEDS VERIFICATION

-- =============================================================================
-- P3-SET-002: UNION DISTINCT
-- =============================================================================
-- TEST: P3-SET-002 - UNION DISTINCT
CREATE TABLE t_set3 (id INT);
CREATE TABLE t_set4 (id INT);
INSERT INTO t_set3 VALUES (1), (2);
INSERT INTO t_set4 VALUES (2), (3);
SELECT * FROM t_set3 UNION SELECT * FROM t_set4 ORDER BY id;
-- EXPECTED: 1, 2, 3
-- STATUS: NEEDS VERIFICATION

-- =============================================================================
-- P3-CHAR-001: CHAR_LENGTH
-- =============================================================================
-- TEST: P3-CHAR-001 - CHAR_LENGTH
SELECT CHAR_LENGTH('hello');
SELECT CHAR_LENGTH('');
SELECT CHAR_LENGTH(NULL);
-- EXPECTED: 5, 0, NULL
-- STATUS: NEEDS VERIFICATION

-- =============================================================================
-- P3-TX-001: Transaction rollback
-- =============================================================================
-- TEST: P3-TX-001 - Transaction rollback
CREATE TABLE t_tx (id INT, val INT);
INSERT INTO t_tx VALUES (1, 10);
BEGIN;
UPDATE t_tx SET val = 20 WHERE id = 1;
ROLLBACK;
SELECT val FROM t_tx WHERE id = 1;
-- EXPECTED: val = 10 (rollback worked)
-- STATUS: NEEDS VERIFICATION

-- =============================================================================
-- P3-SCHEMA-001: sqlite_master
-- =============================================================================
-- TEST: P3-SCHEMA-001 - sqlite_master
CREATE TABLE t_schema (id INT);
SELECT type, name FROM sqlite_master WHERE name = 't_schema';
-- EXPECTED: type='table', name='t_schema'
-- STATUS: NEEDS VERIFICATION

-- =============================================================================
-- P3-COLLATE-001: Collation
-- =============================================================================
-- TEST: P3-COLLATE-001 - Case-insensitive comparison
CREATE TABLE t_collate (name TEXT);
INSERT INTO t_collate VALUES ('Alice'), ('bob'), ('CHARLIE');
SELECT * FROM t_collate WHERE name = 'alice' COLLATE NOCASE;
-- EXPECTED: Returns 'Alice'
-- STATUS: NEEDS VERIFICATION

-- =============================================================================
-- P3-AGG-003: GROUP BY with HAVING
-- =============================================================================
-- TEST: P3-AGG-003 - GROUP BY HAVING
CREATE TABLE t_group (dept TEXT, salary INT);
INSERT INTO t_group VALUES ('eng', 100), ('eng', 200), ('sales', 150);
SELECT dept, SUM(salary) FROM t_group GROUP BY dept HAVING SUM(salary) > 200;
-- EXPECTED: 'eng', 300
-- STATUS: NEEDS VERIFICATION

-- =============================================================================
-- P3-WIN-001: ROW_NUMBER
-- =============================================================================
-- TEST: P3-WIN-001 - ROW_NUMBER
CREATE TABLE t_rownum (id INT, name TEXT);
INSERT INTO t_rownum VALUES (1, 'a'), (2, 'b'), (3, 'c');
SELECT id, ROW_NUMBER() OVER (ORDER BY id) AS rn FROM t_rownum;
-- EXPECTED: 1,1 / 2,2 / 3,3
-- STATUS: NEEDS VERIFICATION

-- =============================================================================
-- P3-INSERT-001: INSERT with DEFAULT
-- =============================================================================
-- TEST: P3-INSERT-001 - INSERT DEFAULT
CREATE TABLE t_insert_default (id INT DEFAULT 0, name TEXT DEFAULT 'unknown');
INSERT INTO t_insert_default () VALUES ();
SELECT * FROM t_insert_default;
-- EXPECTED: 0, 'unknown'
-- STATUS: NEEDS VERIFICATION

-- =============================================================================
-- P3-UPDATE-001: UPDATE with expression
-- =============================================================================
-- TEST: P3-UPDATE-001 - UPDATE expression
CREATE TABLE t_update (id INT, val INT);
INSERT INTO t_update VALUES (1, 10);
UPDATE t_update SET val = val * 2 WHERE id = 1;
SELECT val FROM t_update WHERE id = 1;
-- EXPECTED: val = 20
-- STATUS: NEEDS VERIFICATION

-- =============================================================================
-- P3-DELETE-001: DELETE with subquery
-- =============================================================================
-- TEST: P3-DELETE-001 - DELETE subquery
CREATE TABLE t_del1 (id INT);
CREATE TABLE t_del2 (id INT, del1_id INT);
INSERT INTO t_del1 VALUES (1), (2), (3);
INSERT INTO t_del2 VALUES (10, 1), (20, 2);
DELETE FROM t_del1 WHERE id IN (SELECT del1_id FROM t_del2);
SELECT * FROM t_del1 ORDER BY id;
-- EXPECTED: Only id=3 remains
-- STATUS: NEEDS VERIFICATION
