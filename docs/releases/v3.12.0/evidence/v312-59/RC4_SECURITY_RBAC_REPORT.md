# V312-59-C RC4 — Security / RBAC Report

**Issue**: #4386 (V312-59-C)
**STAGE.yaml key**: `promotion_to_RC_requires[4]` — "Security and role-based access tests pass"
**Verdict**: ✅ PASS
**Wrapper for**: `docs/releases/v3.12.0/evidence/gmp_compliance/V312-53-REPORT.md`

---

## Source evidence

Primary report: `docs/releases/v3.12.0/evidence/gmp_compliance/V312-53-REPORT.md`
- Issue: #4226 [V312-53-gate]
- Generated: 2026-08-14T20:16:02Z
- Baseline commit: `9170661f46d42806f578911a761ff9798ab8f240`
- Anti-Fabrication-Policy-v1.0: applied

## Verbatim key features from V312-53

### ACL role × permission map (5 roles × 11 ops)

- Source: `crates/gmp/src/acl.rs:45` `GmpRole` (Admin/Auditor/Editor/Viewer/BackupOperator)
- Source: `crates/gmp/src/acl.rs:66` `role_permissions(role)`
- Source: `crates/gmp/src/acl.rs:111` `check_permission(role, op)` returns `AccessDecision::Allowed/Denied` (fail-closed)
- Coverage test: `test_acl_full_matrix_5_roles_x_11_operations` — enumerates 5×11=55 cells, hardcodes expected matrix, asserts both `AclContext::can` and `check_permission` agree, verifies 28 allowed + 27 denied invariants
- **12 ACL tests, all PASS**

### Fail-closed semantics

`check_permission(role, op)` returns `AccessDecision::Denied { reason }` for:
- Unrecognised ops (deny-by-default)
- Role lacks permission
- New test `test_permission_guard_fail_closed` PASS

### Audit hash chain integrity

- SHA-256 event_hash → previous_hash chain
- `test_hash_chain_two_rows`, `test_hash_chain_genesis_previous_hash_none`,
  `test_hash_chain_tamper_detection`, `test_hash_chain_tamper_detection_negative_no_mutate` PASS
- `test_compliance_action_variants_roundtrip` PASS — round-trips 6 compliance ops (Import/Export/Approve/Review/Backup/Restore)

## Cross-reference to B8

The B8_THRESHOLDS_OVERRIDE gate references this evidence via
`GMP_AUDIT_TAMPER_TEST_REQUIRED=true` (PASS).

## RC4 verdict for V312-59-C composite gate

```
[4/11] RC4_SECURITY_RBAC
  [EVIDENCE_FILE]      PASS
  [INTEGRATION_TEST]   N/A (12 ACL tests PASS in upstream V312-53)
  → PASS
```

This gate is now PASS for `promotion_to_RC_requires[4]`.