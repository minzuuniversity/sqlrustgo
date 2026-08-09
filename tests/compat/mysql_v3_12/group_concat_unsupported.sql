# name: group_concat_unsupported
# expect: PASS
# NOTE: v3.11.0 server silently accepts GROUP_CONCAT (parsed but
# not executed). Disposition: PASS-with-caveat.
CREATE TABLE concat_t (x INT);
INSERT INTO concat_t VALUES (1);
SELECT GROUP_CONCAT(x) FROM concat_t;
