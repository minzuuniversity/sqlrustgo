# A04 SQL/protocol differential report

corpus_sha256: `ea9fe1cf3adb06296730cbcc31f05f537577ad592073d87b15d7044ab8056c3e`  seed: `410`

| verdict | count |
|---|---|
| match | 22 |
| mismatch | 32 |
| not_comparable | 0 |

## Cases

| id | domain | verdict | detail |
|---|---|---|---|
| A04-TYPE-001 | coltype | mismatch | column types differ: subj=[15, 15, 15, 15, 15] ref=[8, 253, 246, 6, 246]; multiset differs (same count 1): only_subj=[('1', 'x', '3.5', 'NULL', '1')] only_ref=[('1', 'x', '3.5', 'NULL', '1.0')] |
| A04-TYPE-002 | coltype | match | identical (names/types/rows) |
| A04-TYPE-003 | coltype | mismatch | column types differ: subj=[15, 15, 15, 15] ref=[8, 246, 3, 3] |
| A04-TYPE-004 | coltype | match | identical (names/types/rows) |
| A04-TYPE-005 | coltype | match | identical (names/types/rows) |
| A04-TYPE-006 | coltype | match | identical (names/types/rows) |
| A04-TYPE-007 | coltype | match | identical (names/types/rows) |
| A04-TYPE-008 | coltype | mismatch | column types differ: subj=[3, 15, 246, 10] ref=[3, 253, 246, 10]; multiset differs (same count 1): only_subj=[('1', 'a', '1.5', '2024-01-01')] only_ref=[('1', 'a', '1.50', '2024-01-01')] |
| A04-JOIN-001 | join | mismatch | column names differ: subj=['j_a.id', 'j_b.name'] ref=['id', 'name']; column types differ: subj=[3, 15] ref=[3, 253] |
| A04-JOIN-002 | join | mismatch | column names differ: subj=['j_a2.id', 'j_b2.name'] ref=['id', 'name']; column types differ: subj=[3, 15] ref=[3, 253] |
| A04-JOIN-003 | join | mismatch | column names differ: subj=['j_c.id', 'j_d.name'] ref=['id', 'name']; column types differ: subj=[3, 15] ref=[3, 253] |
| A04-JOIN-004 | join | mismatch | column names differ: subj=['a.id', 'b.id'] ref=['id', 'id'] |
| A04-JOIN-005 | join | mismatch | column names differ: subj=['a.id', 'b.name', 'c.tag'] ref=['id', 'name', 'tag']; column types differ: subj=[15, 15, 15] ref=[3, 253, 253] |
| A04-AGG-001 | aggregate | mismatch | column types differ: subj=[15, 15] ref=[3, 8] |
| A04-AGG-002 | aggregate | mismatch | column types differ: subj=[15, 15, 15, 15] ref=[3, 246, 3, 3]; multiset differs (same count 2): only_subj=[('1', '15', '10', '20'), ('2', '10', '5', '15')] only_ref=[('1', '15.0000', '10', '20'), ('2', '10.0000', '5', '15')] |
| A04-AGG-003 | aggregate | mismatch | column types differ: subj=[15, 15] ref=[8, 246] |
| A04-AGG-004 | aggregate | mismatch | column types differ: subj=[15, 15, 15] ref=[3, 3, 8] |
| A04-AGG-005 | aggregate | mismatch | column types differ: subj=[15] ref=[8] |
| A04-HAVING-001 | having | mismatch | column types differ: subj=[15, 15] ref=[3, 8] |
| A04-HAVING-002 | having | mismatch | column types differ: subj=[15, 15] ref=[3, 246] |
| A04-HAVING-003 | having | mismatch | subject errored (code 2013) but other returned rows: Lost connection to MySQL server during query (The read operation timed out) |
| A04-SUBQ-001 | subquery | match | identical (names/types/rows) |
| A04-SUBQ-002 | subquery | match | identical (names/types/rows) |
| A04-SUBQ-003 | subquery | match | identical (names/types/rows) |
| A04-SUBQ-004 | subquery | mismatch | column types differ: subj=[3, 15] ref=[3, 3] |
| A04-NULL-001 | null | match | identical (names/types/rows) |
| A04-NULL-002 | null | match | identical (names/types/rows) |
| A04-NULL-003 | null | match | identical (names/types/rows) |
| A04-NULL-004 | null | mismatch | column types differ: subj=[3, 15] ref=[3, 5] |
| A04-NULL-005 | null | mismatch | column types differ: subj=[3, 15] ref=[3, 8] |
| A04-NULL-006 | null | match | identical (names/types/rows) |
| A04-ORDER-001 | order_limit | match | identical (names/types/rows) |
| A04-ORDER-002 | order_limit | match | identical (names/types/rows) |
| A04-ORDER-003 | order_limit | match | identical (names/types/rows) |
| A04-ORDER-004 | order_limit | mismatch | multiset differs (same count 2): only_subj=[('5',)] only_ref=[('7',)] |
| A04-CAST-001 | typeconv | mismatch | column types differ: subj=[15, 15] ref=[8, 8] |
| A04-CAST-002 | typeconv | match | identical (names/types/rows) |
| A04-CAST-003 | typeconv | mismatch | column types differ: subj=[3, 15] ref=[3, 253] |
| A04-CAST-004 | typeconv | mismatch | column types differ: subj=[3, 15] ref=[3, 253] |
| A04-CAST-005 | typeconv | mismatch | rowcount differs: subj=0 ref=1 |
| A04-ERR-001 | error | mismatch | both error, DIFFERENT codes: subj=1105 ref=1054 |
| A04-ERR-002 | error | match | both error, same code 1146 |
| A04-ERR-003 | error | match | both error, same code 1064 |
| A04-ERR-004 | error | mismatch | reference errored (code 1146) but other returned rows: Table 'a04_diff.nonexist_tbl' doesn't exist |
| A04-COL-001 | colname | mismatch | column types differ: subj=[3, 15] ref=[3, 253] |
| A04-COL-002 | colname | mismatch | column types differ: subj=[3, 15] ref=[3, 253] |
| A04-COL-003 | colname | mismatch | column names differ: subj=['BinaryOp(Identifier("a"), "+", Identifier("b"))'] ref=['a + b']; column types differ: subj=[15] ref=[8] |
| A04-COL-004 | colname | match | identical (names/types/rows) |
| A04-DDL-001 | ddl | mismatch | column types differ: subj=[3, 15, 246] ref=[3, 253, 246]; multiset differs (same count 1): only_subj=[('1', 'a', '1.5')] only_ref=[('1', 'a', '1.50')] |
| A04-DDL-002 | ddl | mismatch | result kind differs: subj=ok ref=resultset |
| A04-DDL-003 | ddl | mismatch | column types differ: subj=[15] ref=[8] |
| A04-CONSTRAINT-001 | constraint | match | both error, same code 1062 |
| A04-CONSTRAINT-002 | constraint | match | side-effect(affected): subj=1 ref=1 |
| A04-CONSTRAINT-003 | constraint | match | identical (names/types/rows) |
