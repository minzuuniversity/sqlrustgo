# name: prepared_stmt_roundtrip
# expect: DEFERRED: v3.12 follow-up
# NOTE: v3.11.0 server does NOT implement PREPARE / EXECUTE
# binary protocol syntax. The server returns:
#   "Execution error: prepared statement 'stmt' not found
#    (call PREPARE first)"
# This is a genuine server-side gap, not a runner / fixture
# issue. Tracked in the V312-15 (CREATE SEQUENCE) / V312-19
# (corpus) work. The fixture is recorded as `deferred` until
# the server-side PREPARE support lands.
PREPARE stmt FROM 'SELECT 1';
EXECUTE stmt;
