# name: with_cube_unsupported
# expect: UNSUPPORTED: WITH CUBE not implemented
SELECT a, COUNT(*) FROM t GROUP BY a WITH CUBE;
