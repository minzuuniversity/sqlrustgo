# Contributing to SQLRustGo

> **Maintainer**: Hermes C (openclaw@gaoyuanyiyao.com)
> **Status**: Active
> **Last Updated**: 2026-06-03

Thank you for your interest in contributing to SQLRustGo! This document provides a quick overview of how to contribute effectively.

## Quick Links

- [Architecture Decisions](docs/governance/adr/) — Design rationale
- [Release Lifecycle](docs/governance/RELEASE_LIFECYCLE.md) — Alpha/Beta/RC/GA process
- [Version History](docs/releases/VERSION_HISTORY.md) — All past versions
- [Current Development](docs/releases/v3.8.0/) — v3.8.0 docs

## Development Setup

```bash
# Clone the repo
git clone https://github.com/minzuuniversity/sqlrustgo.git
cd sqlrustgo

# Install Rust (1.75+ recommended)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build the workspace
cargo build --all-features

# Run tests
cargo test --all-features
```

## Development Workflow

1. **Pick an issue** from the Gitea issue tracker (or create one)
2. **Branch** from `develop/v3.8.0`: `git checkout -b fix/<descriptive-name>`
3. **Implement** + test + commit (`git commit -m "fix(scope): description"`)
4. **Push** to fork or Gitea
5. **Open PR** against `develop/v3.8.0`

## Code Style

- Follow `rustfmt` defaults: `cargo fmt --all`
- Pass `cargo clippy --all-features -- -D warnings` (zero warnings)
- Document public APIs with `///` doc comments
- Add unit tests for new functionality

## Commit Message Convention

```
<type>(<scope>): <short description>

[body - detailed explanation]

[footer - references to issues]
```

**Types**: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`
**Scopes**: `executor`, `storage`, `parser`, `planner`, `wal`, `gate`, `docs`

Example:
```
fix(wal): correct LSN assignment in WalStorage commit path

Previously, all WAL entries were assigned lsn=0, causing
checkpoint advance to never trigger. Add a per-instance LSN
counter and route all append operations through it.

Fixes #2588
```

## Testing

```bash
# Run unit tests
cargo test --lib

# Run doc tests
cargo test --doc

# Run specific test
cargo test -p sqlrustgo-executor test_name

# Coverage (requires cargo-llvm-cov)
cargo llvm-cov --all-features --workspace
```

## Pull Request Process

1. Update `CHANGELOG.md` if your change is user-facing
2. Ensure CI passes (clippy + tests + format)
3. Request review from a maintainer
4. Squash-merge after approval

## Governance

- All changes go through **Alpha → Beta → RC → GA** gates
- Architecture changes require ADR (Architecture Decision Record)
- Truthfulness framework applies — never fabricate test results

See `docs/governance/` for full governance framework.

## License

By contributing, you agree your contributions will be licensed under the same license as the project.

## Questions?

Open an issue or contact the maintainers.
