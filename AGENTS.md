# AGENTS.md

Guidance for agents working in this repo. `CLAUDE.md` has a fuller module-by-module reference; this file holds the non-obvious, verified facts.

## Project

`aigitcommit` — single-crate Rust CLI (edition 2024) that generates Conventional Commits messages from staged diffs via OpenAI-compatible APIs. Binary target is `src/main.rs`; all logic lives in the library crate (`src/lib.rs`).

## Commands (match CI exactly)

```bash
cargo fmt --all -- --check                              # then `cargo fmt --all` to fix
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --locked
cargo package --locked --allow-dirty                    # CI runs this; keep Cargo.toml metadata valid
```

CI gates: lint + `cargo audit` must pass before the test matrix (stable/beta; nightly allowed to fail). Run fmt/clippy/tests before considering work done.

Local smoke checks (require `OPENAI_API_TOKEN` / `OPENAI_API_BASE` / `OPENAI_MODEL_NAME`):

```bash
cargo run -- --check-env      # verify env vars
cargo run -- --check-model    # verify API/model reachable
```

## Non-obvious architecture

- `main.rs` imports from the library crate (`use aigitcommit::...`). **New modules must be declared in `src/lib.rs`** or the binary cannot see them.
- Prompts: `templates/system.txt` is embedded with `include_str!` in `main.rs`; `templates/user.txt` is an Askama template compiled at build time. Template edits require a rebuild; Askama reports template syntax errors at compile time.
- `build.rs` (via the `built` crate) generates `built_info` (`PKG_NAME`, `PKG_VERSION`, ...) which `cli.rs` uses for clap's name/version/about. Don't hardcode version strings.
- Git operations use `git2`/libgit2 deliberately — **do not shell out to the `git` CLI** when extending functionality.
- Diff noise filtering: lock files (`Cargo.lock`, `package-lock.json`, `go.sum`, ...) are excluded via `EXCLUDED_FILES` in `src/git/repository.rs`.
- Response cache (`src/cache.rs`): stored under `<repo>/.git/aigitcommit-cache/`, keyed by FNV-1a of (model, system prompt, diff lines, recent logs). When testing prompt or diff-handling changes, pass `--no-cache` or you'll get stale cached responses.
- Error handling convention: `utils::Result<T>` = `Result<T, Box<dyn Error>>`.
- AI response contract: the model must return `title\n\nbody` (split on the first double newline); `GitMessage` Display reassembles it the same way.

## Testing quirks

- `openai::test::test_prompt` **silently passes** if `TEST_REPO_PATH` is unset — it's a no-op unless run as `TEST_REPO_PATH=/path/to/repo cargo test test_prompt`. The repo must have staged changes for it to assert anything meaningful.
- Some tests in `src/git/message.rs` fall back to `"."` when `TEST_REPO_PATH` is unset, so results depend on the working directory.
- Tests mutate process env (`std::env::set_var`); they use unique keys, so keep that pattern if adding env-dependent tests.

## Style conventions

- Every source file starts with a `/*! ... */` (or `/* ... */`) header block containing copyright, `File:`, `Author:`, `File Created:`, `Modified By:`/`Last Modified:` fields. Match this when creating new files; it is a manual convention (no formatter enforces it).
- Logging: use `tracing` (`debug!`, `trace!`, `warn!`), not `log`/`println!`, inside library code; CLI user-facing output goes through `utils` formatting.
- Both `log` and `tracing` crates are dependencies, but `tracing` is what the code actually uses.

## Branching & release

Git-flow: `main` = releases, `develop` = integration, plus `feature/**` and `release/v*` branches. CI runs on all of them. Publishing to crates.io happens only from `main` pushes or `v*.*.*` tags (`.github/workflows/crates.yml`); `Cargo.toml` version must be bumped for a release.
