# AGENTS.md

Non-obvious, verified guidance for agents working in this repository. Use the
[README](README.md) for user-facing setup and configuration. `CLAUDE.md`
currently contains unresolved merge-conflict markers; treat this file, source,
and CI workflows as authoritative until it is repaired.

## Project

`aigitcommit` is a single-crate Rust 2024 CLI that generates Conventional
Commit messages from staged diffs through OpenAI-compatible APIs. `src/main.rs`
owns CLI orchestration; reusable modules are exported by `src/lib.rs`.

## Commands (match CI exactly)

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo audit
cargo test --all --locked
cargo package --locked --allow-dirty
```

These match [.github/workflows/cargo.yml](.github/workflows/cargo.yml). Lint and
audit gate the stable/beta/nightly test matrix; nightly is allowed to fail.
Run `cargo fmt --all` to fix formatting.

Local smoke checks:

```bash
cargo run -- --check-env
cargo run -- --check-model
```

`--check-env` only reports configuration and needs no credentials.
`--check-model` needs credentials and a reachable endpoint; API base and model
have defaults. See [README.md](README.md#environment-variables) for variables.

## Non-obvious architecture

- `main.rs` imports from the library crate (`use aigitcommit::...`). Declare new
	modules in `src/lib.rs` or the binary cannot import them.
- `templates/system.txt` is embedded with `include_str!`; `templates/user.txt`
	is compiled by Askama. Template edits require a rebuild.
- `build.rs` generates `built_info`; use it for package metadata rather than
	hardcoding names or versions.
- Diff, history, and commits use `git2`. The intentional exception is
	`Repository::git_cli_config`, which invokes `git config --get` because
	libgit2 does not resolve newer conditional includes identically.
- `Repository::get_diff` skips only filenames in `EXCLUDED_FILES`, omits binary
	content, and reads `AIGITCOMMIT_DIFF_CONTEXT_LINES`.
- OpenAI request tuning is model-aware and environment-controlled in
	`OpenAI::request_tuning`. Keep provider-specific fields optional for
	OpenAI-compatible endpoints; document new controls in the README.
- Response cache lives under `<repo>/.git/aigitcommit-cache/`. Its key includes
	model, request profile, system prompt, diff, and recent logs. Use `--no-cache`
	when testing prompt, request-option, or diff-handling changes.
- The CLI accepts a repository directory or subdirectory and discovers upward.
	`install-hook` differs: its target itself must contain `.git`.
- Error handling convention: `utils::Result<T>` = `Result<T, Box<dyn Error>>`.
- AI responses must be `title\n\nbody`; `main.rs` splits on the first blank
	line and `GitMessage` reassembles the same format.

## Testing quirks

- `openai::test::test_prompt` **silently passes** if `TEST_REPO_PATH` is unset — it's a no-op unless run as `TEST_REPO_PATH=/path/to/repo cargo test test_prompt`. The repo must have staged changes for it to assert anything meaningful.
- Some tests in `src/git/message.rs` fall back to `"."` when `TEST_REPO_PATH` is unset, so results depend on the working directory.
- Tests mutate process environment. Prefer unique variable names or serialize
	tests that touch shared production keys; Rust 2024 marks mutation as unsafe.
- Known mismatch: `Repository::get_logs` intentionally returns an empty list
	for an unborn branch, while `get_logs_errors_on_unborn_branch` currently
	expects an error. Do not change production behavior merely to satisfy that
	stale assertion.

## Style conventions

- Preserve the existing source header-block style when creating files; it is a
	manual convention and varies slightly between older files.
- Use `tracing` for new operational diagnostics. User-facing CLI output belongs
	on stdout or in `utils`; do not introduce new `log` usage.

## Branching & release

Git-flow uses `main` for releases, `develop` for integration, and `feature/**`
or `release/v*` branches. Publishing behavior is defined by
[.github/workflows/crates.yml](.github/workflows/crates.yml); release versions
come from `Cargo.toml`.
