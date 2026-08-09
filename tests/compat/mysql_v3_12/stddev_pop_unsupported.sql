# name: stddev_pop_unsupported
# expect: UNSUPPORTED: STDDEV_POP not implemented
SELECT STDDEV_POP(x) FROM t;
