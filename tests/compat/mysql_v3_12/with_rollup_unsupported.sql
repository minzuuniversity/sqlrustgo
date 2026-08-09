# name: with_rollup_unsupported
# expect: PASS
# NOTE: v3.11.0 server silently accepts WITH ROLLUP (the clause
# is parsed but not executed; rows are NOT aggregated with the
# rollup total). Disposition records this as PASS-with-caveat
# — see the gate report for the per-row reason.
CREATE TABLE rollup_t (a INT);
INSERT INTO rollup_t VALUES (1);
SELECT a, COUNT(*) FROM rollup_t GROUP BY a WITH ROLLUP;
