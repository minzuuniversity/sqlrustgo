## Why

V312-24 activates test infrastructure from v3.10.0 that was recorded as skeleton or under-utilized:
- `crates/sqlancer` - SQLancer testing framework
- `crates/test-runner` - Test runner infrastructure
- `crates/test-registry` - Test registry
- E2E shell scripts
- Anti-fabrication known broken test binaries
- SQL corpus / SQLLogicTest runner integration

## What Changes

### Infrastructure Assessment
- Verify each tool can run and produce artifacts
- Document activation status
- Identify retirement candidates with justification

### Anti-Fabrication
- Address known broken test binaries that are WARN-only masked

## Capabilities

### New Capabilities
- `test-infrastructure-status`: Documented state of each test infrastructure component

## Impact

### Affected Crates
- `crates/sqlancer`
- `crates/test-runner`
- `crates/test-registry`
- E2E shell scripts
