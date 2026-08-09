# name: with_cube_unsupported
# expect: PASS
# NOTE: v3.11.0 server silently accepts WITH CUBE (same caveat
# as with_rollup_unsupported). Disposition: PASS-with-caveat.
CREATE TABLE cube_t (a INT);
INSERT INTO cube_t VALUES (1);
SELECT a, COUNT(*) FROM cube_t GROUP BY a WITH CUBE;
