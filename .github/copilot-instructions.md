# Copilot Instructions

`aigitcommit` is a single-crate Rust 2024 CLI that generates Conventional Commit messages from staged Git changes via OpenAI-compatible APIs. Refer to [README.md](../README.md) for user-facing CLI flags, environment variables, and model tuning tables.

## Build, Test, and Lint Commands

Match the CI workflow in [`.github/workflows/cargo.yml`](./workflows/cargo.yml):

```bash
# Format check & auto-format
cargo fmt --all -- --check
cargo fmt --all

# Lint (CI gates on zero warnings with --locked)
cargo clippy --all-targets --all-features --locked -- -D warnings

# Security audit
cargo audit

# Run the full test suite (unit + integration tests in tests/generation.rs)
cargo test --all --locked

# Run a single unit test by name (with stdout/stderr shown)
cargo test <test_name> -- --nocapture

# Run a single integration test in tests/generation.rs
cargo test --test generation <test_name> -- --nocapture

# Verify crates.io packaging
cargo package --locked --allow-dirty
```

Local smoke checks:

```bash
# Check environment variable presence without revealing values (no credentials needed)
cargo run -- --check-env

# Verify API connectivity and model availability (requires credentials)
cargo run -- --check-model
```

## High-Level Architecture

- **Library + Binary Split**: [`src/lib.rs`](../src/lib.rs) exports all reusable modules (`built_info`, `cache`, `cli`, `git`, `openai`, `utils`), and [`src/main.rs`](../src/main.rs) owns CLI orchestration (`use aigitcommit::...`). Any new module must be declared in `src/lib.rs` to be accessible from `main.rs` or integration tests.
- **Build Metadata**: [`build.rs`](../build.rs) generates `built_info` (via the `built` crate) and sets `AIGITCOMMIT_BUILD_DATE` (`YYYYMMDD`). Always read package metadata (`PKG_NAME`, `PKG_VERSION`, `PKG_DESCRIPTION`, `PKG_AUTHORS`, `PKG_HOMEPAGE`) from `built_info` rather than hardcoding strings.
- **Staged Diff & Context Budgeting (`src/git/repository.rs`)**:
  - Uses `git2` (`diff_tree_to_index` with exact-match rename detection) to inspect staged changes only.
  - Lockfiles in `EXCLUDED_FILES` (`go.sum`, `Cargo.lock`, `package-lock.json`, `yarn.lock`, `pnpm-lock.yaml`) and binary files omit patch bodies, but their file metadata (status, paths, blob OIDs, modes) is always retained.
  - Serialized diff output is bounded by `DEFAULT_MAX_DIFF_BYTES` (`24,000` bytes by default, minimum `4,096`, configurable via `--max-diff-bytes` / `AIGITCOMMIT_MAX_DIFF_BYTES`). After reserving space for all file metadata, the remaining budget is split across files, hunks, and line groups (`-`, `+`, context) so large replacements or early files never starve later files.
- **Prompt Construction & Injection Defenses (`src/openai.rs`, `templates/`)**:
  - [`templates/system.txt`](../templates/system.txt) is embedded via `include_str!` in `main.rs`; [`templates/user.txt`](../templates/user.txt) is compiled by Askama. Editing either template requires a rebuild.
  - `OpenAI::prompt_with_instructions` serializes up to 5 distinct recent commit subjects (truncated to 200 chars each), staged diff lines, and user `--instructions` as JSON strings inside `{"guidance":...,"subjects":...,"staged":...}` so repository content remains strictly data.
  - Output language is appended to the system prompt as a JSON-encoded string and takes precedence over user guidance or repository content.
- **Model-Aware Request Tuning (`src/openai.rs`)**:
  - `OpenAI::request_with_options` applies model- and endpoint-aware defaults for `reasoning_effort` and `max_completion_tokens` (e.g., `low` / `2048` for GPT-5 non-chat/pro and `grok-4.5`..`4.7`; ZenMux-specific profiles only when `OPENAI_API_BASE` is `https://zenmux.ai/...`).
  - Keep optional request fields omittable (`OPENAI_API_REASONING_EFFORT=off`, `OPENAI_API_MAX_TOKENS=0`) for strict OpenAI-compatible providers.
- **Response Normalization, Validation & Trailers (`src/git/message.rs`, `src/utils.rs`)**:
  - `GitMessage::normalize_response` normalizes CRLF, strips a single outer plain-text Markdown fence (` ``` `, ` ```text `, ` ```plaintext `), tolerates a missing blank line between a valid Conventional Commit header and bullet body, and validates 1–5 `- ` bullet lines.
  - `utils::validate_commit_text` rejects control characters and Unicode bidirectional/spoofing codepoints while preserving ZWNJ/ZWJ (`U+200C`/`U+200D`) for legitimate scripts and emoji sequences.
  - Model output must never contain trailers (`Signed-off-by:`, `Co-authored-by:`, `Generated-by:`). `Signed-off-by` and optional `Generated-by: aigitcommit v<version>` (`--attribution`, effective only when signoff is enabled) are appended locally in `GitMessage::new` after response validation.
- **Request-Scoped Cache (`src/cache.rs`)**:
  - Stored under `<repo>/.git/aigitcommit-cache/` using a 64-bit FNV-1a hash (`Cache::build_request_key`) over `"request-v2"`, `api_base`, and the exact serialized `CreateChatCompletionRequest` (excluding API credentials).
  - Because staged file metadata includes blob OIDs, changes inside omitted lockfiles or truncated patch regions still invalidate the cache. Because trailers are applied after cache lookup, toggling `--signoff` or `--attribution` reuses cached generation. Use `--no-cache` when testing prompt, diff, or request-tuning changes.

## Key Conventions & Non-Obvious Patterns

- **libgit2 vs. `git` CLI Exception**: All Git operations use `git2` except `Repository::git_cli_config`, which intentionally shells out to `git -C <workdir> config --get <key>` first (before falling back to `git2::Config` and env vars) because `libgit2` does not evaluate newer `includeIf` conditional includes (such as `hasconfig:remote.*.url:...`).
- **Repository Path Resolution**: Normal runs discover the repository upward from subdirectories (`Repository::new` uses `Repository::open_ext`). By contrast, `install-hook` requires the target path itself to contain a `.git` directory and backs up any existing hook to `prepare-commit-msg.bak.<unix-timestamp>[.<counter>]`.
- **Unborn Branch Handling**: `Repository::get_logs` and `Repository::get_diff_with_options` intentionally treat `git2::ErrorCode::UnbornBranch` as empty history / initial commit rather than an error so the tool works on brand-new repositories before the first commit.
- **Error Handling & Logging**:
  - Use `utils::Result<T>` (`Result<T, Box<dyn std::error::Error>>`) for fallible functions.
  - Use `tracing` (`trace!`, `debug!`, `info!`, `warn!`) for diagnostics—never introduce `log` crate macros. Diagnostics and interactive spinners (`cliclack`) write to `stderr` so `stdout` stays clean for `--json` and piped output.
- **Testing Quirks**:
  - `openai::test::test_prompt` is a no-op unless `TEST_REPO_PATH` is set to a repository with staged changes (`TEST_REPO_PATH=/path/to/repo cargo test test_prompt`).
  - Rust 2024 marks `std::env::set_var` / `remove_var` as `unsafe`. When writing unit tests that mutate environment variables, use unique variable names or subprocess isolation (like `TestRepo` + `MockApi` in [`tests/generation.rs`](../tests/generation.rs)) to avoid parallel test races.
- **Source File Headers**: Preserve the existing copyright/metadata comment header block when creating or editing Rust source files.
- **Branching & Releases**: Follows Git-flow (`main` for releases, `develop` for integration, `feature/**` branches). Release tags (`v*`) must match the `version` in `Cargo.toml` as enforced by [`.github/workflows/release.yml`](./workflows/release.yml).
