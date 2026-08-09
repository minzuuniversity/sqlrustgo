# name: with_rollup_unsupported
# expect: UNSUPPORTED: WITH ROLLUP not implemented
SELECT a, COUNT(*) FROM t GROUP BY a WITH ROLLUP;
