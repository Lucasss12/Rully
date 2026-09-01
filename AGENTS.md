# Rully

Rully is a terminal-first HTTP client written in Rust.

The project starts as a CLI and will eventually evolve into a TUI. The goal is to build a small, useful developer tool while learning idiomatic Rust through real-world development.

See [ROADMAP.md](./roadmap.md) for the planned features and project phases.

## Commands

```sh
cargo build        # build
cargo run          # run
cargo test         # run tests
cargo clippy -- -D warnings  # lint (warnings = errors)
cargo fmt          # format
```

## Commit Convention

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>
```

Types:
- `feat` — new feature
- `fix` — bug fix
- `refactor` — code change that neither fixes a bug nor adds a feature
- `docs` — documentation only
- `chore` — build, CI, dependencies, or other maintenance
- `test` — adding or updating tests

Examples:
```
feat: add request body parsing
fix: handle empty response headers
refactor: extract header validation into its own module
chore: add GitHub Actions CI
```

## Development Rules

* Prefer simple, idiomatic Rust.
* Avoid premature abstractions.
* Avoid unnecessary dependencies.
* Do not implement future roadmap features unless explicitly requested.
* Keep changes focused on the requested feature.
* Prefer borrowing over unnecessary cloning.
* Avoid `unwrap()` and `expect()` unless the invariant is genuinely guaranteed.
* Use `Result` and `Option` appropriately.
* Do not clone values just to satisfy the borrow checker without understanding why.
* Explain non-obvious Rust patterns when introducing them.