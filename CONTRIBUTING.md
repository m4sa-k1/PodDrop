# Contributing to PodDrop

Thank you for your interest in contributing to PodDrop!

## Development Principles

1. **Step-by-step verification**: Core database read/write features are verified in `poddrop-core` and `poddrop-cli` before exposing them in `poddrop-gui`.
2. **Never commit real device data or media**: Do not commit `iTunesDB`, `ArtworkDB`, `SysInfo*`, `.ithmb`, or audio/image test fixtures from real devices. Use synthetic/mock structures for tests.
3. **Code quality**: All PRs must pass:
   - `cargo fmt --all -- --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`

## Commit Convention

We follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` — New feature
- `fix:` — Bug fix
- `docs:` — Documentation changes
- `refactor:` — Code refactoring without functional changes
- `test:` — Adding or updating tests
- `chore:` — Build, CI, dependency, or tooling updates
- `i18n:` — Translation / localization updates
