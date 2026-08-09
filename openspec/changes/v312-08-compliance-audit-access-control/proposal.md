## Why

V312-08 maps SQLRustGo behavior to GMP/ALCOA+ controls:
- Audit trail for import/search/export/approve/backup/restore/review
- ACL covering SQL/vector/graph/GMP retrieval
- Tamper and unauthorized access tests fail closed

## What Changes

### Audit Trail
- Verify audit hash chain for all GMP operations
- Document current audit implementation

### Access Control
- Verify ACL coverage for SQL, vector, graph, GMP retrieval
- Verify tamper tests fail closed

## Capabilities

### New Capabilities
- `audit-trail-verification`: Documented audit hash chain status
- `acl-verification`: Documented ACL coverage

## Impact

### Affected Modules
- `crates/security` - Audit and ACL
- `crates/catalog` - Access control
