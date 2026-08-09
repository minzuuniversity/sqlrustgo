# v3.12.0 RC/GA Reviewer Sign-off

> **Template version**: 1.0
> **Status**: DRAFT (V312-19 / ISSUE #3906)
> **Required for**: v3.12.0 RC → GA promotion

This template MUST be filled and attached (as a PR comment) to every
v3.12.0 RC sign-off. The CI gate
`scripts/gate/assert_reviewer_signoff.sh` validates the structure and
`scripts/gate/check_v312_19_release_gates.sh` checks freshness
(modified within 7 days of HEAD).

---

## Header

- **Issue**: <number>
- **Branch**: <branch> (must match `git rev-parse --abbrev-ref HEAD`)
- **Commit**: <sha> (must match `git rev-parse HEAD`)
- **Gate report**: <path under `docs/releases/v3.12.0/evidence/...>
- **Evidence hash**: <sha256 of the gate report file>
- **Date**: <ISO8601>

## Reviewer A

- **Login**: <gitea login>
- **Command output**: <link to log or inline excerpt>
- **Timestamp**: <ISO8601>
- **source_agent**: <agent id>
- **source_run**: <run id>
- **Output location**: <path>
- **Signature**: <!-- gitea login + comment URL -->

## Reviewer B

- **Login**: <gitea login>
- **Command output**: <link to log or inline excerpt>
- **Timestamp**: <ISO8601>
- **source_agent**: <agent id>
- **source_run**: <run id>
- **Output location**: <path>
- **Signature**: <!-- gitea login + comment URL -->

---

> Both reviewers must be distinct gitea logins. Submit by posting this
> file as a PR comment on the relevant issue.
