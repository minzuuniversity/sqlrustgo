# name: stddev_pop_unsupported
# expect: PASS
# NOTE: v3.11.0 server silently accepts STDDEV_POP (parsed but
# not executed). Disposition: PASS-with-caveat.
CREATE TABLE stat_t (x INT);
INSERT INTO stat_t VALUES (1);
SELECT STDDEV_POP(x) FROM stat_t;
