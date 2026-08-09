## ADDED Requirements

### Requirement: Template structure MUST be machine-checkable

`docs/governance/REVIEWER_SIGNOFF_TEMPLATE.md` SHALL contain a markdown
form with these sections, in this order:

1. `# v3.12.0 RC/GA Reviewer Sign-off`
2. Issue (number)
3. Branch (must match `git rev-parse --abbrev-ref HEAD`)
4. Commit (must match `git rev-parse HEAD`)
5. Gate report (path under `docs/releases/v3.12.0/evidence/`)
6. Evidence hash (SHA256 of the gate report file)
7. Date (ISO8601)
8. `## Reviewer A` block with: login, command output link, timestamp,
   source_agent, source_run, output location, signature (gitea login + comment URL)
9. `## Reviewer B` block (same fields, distinct login)
10. Footer: "Both reviewers must be distinct gitea logins. Submit by
    posting this file as a PR comment on the relevant issue."

#### Scenario: Template renders

- **WHEN** an opener copies the template and fills the required fields
- **THEN** `assert_reviewer_signoff.sh` SHALL parse the file, assert the
  branch and commit match the current working tree, and assert two
  distinct reviewer logins are present

### Requirement: assert_reviewer_signoff.sh MUST validate the form

`scripts/gate/assert_reviewer_signoff.sh <path>` SHALL:

1. Read the sign-off file at `<path>`.
2. Assert the `## Reviewer A` and `## Reviewer B` sections each contain
   a `Login:` line.
3. Assert the two logins differ (case-sensitive).
4. Assert the `Commit:` line matches `git rev-parse HEAD`.
5. Assert the `Branch:` line matches `git rev-parse --abbrev-ref HEAD`.
6. Exit 0 on success; exit non-zero with a clear message otherwise.

#### Scenario: Same reviewer twice fails

- **WHEN** both Reviewer A and Reviewer B have `Login: alice`
- **THEN** the script SHALL exit non-zero with
  "Reviewer A and Reviewer B must be distinct"

#### Scenario: Wrong commit fails

- **WHEN** the sign-off file lists `Commit: 1234` but HEAD is `5678`
- **THEN** the script SHALL exit non-zero with
  "Commit SHA does not match HEAD"
