/*!
 * Copyright (c) 2026 mingcheng <mingcheng@apache.org>
 *
 * This source code is licensed under the MIT License,
 * which is located in the LICENSE file in the source tree's root directory.
 *
 * Author: mingcheng <mingcheng@apache.org>
 * File Created: 2025-03-01 17:17:30
 *
 * Modified By: mingcheng <mingcheng@apache.org>
 * Last Modified: 2026-05-07 11:39:56
 */

use aigitcommit::built_info::{PKG_NAME, PKG_VERSION};
use aigitcommit::cache::Cache;
use aigitcommit::cli::{Cli, Command};
use aigitcommit::git::message::{GitMessage, GitMessageConfig};
use aigitcommit::git::repository::Repository;
use aigitcommit::openai::OpenAI;
use arboard::Clipboard;
use async_openai::types::chat::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};
use clap::Parser;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{Level, debug, error, info, trace};

use aigitcommit::utils::{
    self, OutputFormat, check_env_variables, env, install_hook, save_to_file, should_signoff,
};

// Defaults and embedded resources.
const DEFAULT_MODEL: &str = "gpt-5";
const DEFAULT_LOG_COUNT: usize = 5;
const SYSTEM_PROMPT: &str = include_str!("../templates/system.txt");
const HOOK_NAME: &str = "prepare-commit-msg";
const HOOK_CONTENT: &str = include_str!("../hooks/prepare-commit-msg");

#[tokio::main]
async fn main() -> utils::Result<()> {
    let cli = Cli::parse();
    init_logging(cli.verbose);

    // Subcommands run to completion and never fall through to message generation.
    if let Some(Command::InstallHook { repo_path }) = &cli.command {
        trace!("install-hook subcommand invoked");
        install_hook(repo_path, HOOK_NAME, HOOK_CONTENT)?;
        println!("git hook `{HOOK_NAME}` has been installed successfully.");
        return Ok(());
    }

    let model_name = env::get("OPENAI_MODEL_NAME", DEFAULT_MODEL);
    let client = OpenAI::new();

    // Diagnostic flags also short-circuit before touching the repository.
    if cli.check_env {
        debug!("model name: `{model_name}`");
        check_env_variables();
        return Ok(());
    }
    if cli.check_model {
        debug!("model name: `{model_name}`");
        check_model_availability(&client, &model_name).await?;
        return Ok(());
    }

    let repo_dir = resolve_repo_dir(&cli.repo_path)?;
    trace!("specified repository directory: {repo_dir:?}");
    let repository = Repository::new(
        repo_dir
            .to_str()
            .ok_or("invalid UTF-8 in repository path")?,
    )?;

    let cache = Cache::new(repository.git_dir());
    if cli.clear_cache {
        let n = cache
            .clear()
            .map_err(|e| format!("failed to clear cache: {e}"))?;
        info!("cleared {n} cache entries");
        writeln!(std::io::stdout(), "cleared {n} cache entries.")?;
        return Ok(());
    }

    let diffs = repository.get_diff()?;
    debug!("got diff size is {}", diffs.len());
    if diffs.is_empty() {
        return Err("no changes found in the repository".into());
    }

    let logs = repository.get_logs(DEFAULT_LOG_COUNT)?;
    debug!("got logs size is {}", logs.len());
    if logs.is_empty() {
        return Err("no commit history found in the repository".into());
    }

    let raw = generate_message(&client, &cache, &model_name, &logs, &diffs, cli.no_cache).await?;
    let (title, content) = raw
        .split_once("\n\n")
        .ok_or("Invalid response format: expected title and content separated by double newline")?;

    let need_signoff = should_signoff(&repository, cli.signoff);
    let message = GitMessage::new(
        &repository,
        GitMessageConfig::new(title, content, need_signoff),
    )?;

    OutputFormat::detect(cli.json, cli.no_table).write(&message)?;

    if cli.copy_to_clipboard {
        copy_to_clipboard(&message)?;
    }
    if cli.commit {
        run_commit_flow(&repository, &message, cli.yes)?;
    }
    if !cli.save.is_empty() {
        match save_to_file(&cli.save, &message) {
            Ok(()) => info!("commit message saved to file: {}", cli.save),
            Err(e) => error!(
                "failed to save commit message to file `{}`: {}",
                cli.save, e
            ),
        }
    }

    Ok(())
}

/// Canonicalize the user-supplied repository path and verify it is a directory.
fn resolve_repo_dir(input: &str) -> utils::Result<PathBuf> {
    let dir = fs::canonicalize(Path::new(input))
        .map_err(|e| format!("failed to resolve repository path `{input}`: {e}"))?;
    if !dir.is_dir() {
        return Err(format!("the specified path is not a directory: {dir:?}").into());
    }
    Ok(dir)
}

/// Look up a cached completion if allowed; otherwise call the API and persist
/// the result.
async fn generate_message(
    client: &OpenAI,
    cache: &Cache,
    model_name: &str,
    logs: &[String],
    diffs: &[String],
    no_cache: bool,
) -> utils::Result<String> {
    let key = Cache::build_key(model_name, SYSTEM_PROMPT, diffs, logs);
    debug!("cache key: {key}");

    if no_cache {
        trace!("--no-cache enabled, skipping cache lookup");
    } else if let Some(cached) = cache.get(&key) {
        info!("reusing cached commit message (key: {key})");
        return Ok(cached);
    }

    let fresh = request_completion(client, model_name, logs, diffs).await?;
    if !no_cache {
        cache.put(&key, &fresh);
    }
    Ok(fresh)
}

/// Push the rendered commit message onto the system clipboard.
fn copy_to_clipboard(message: &GitMessage) -> utils::Result<()> {
    let mut clipboard =
        Clipboard::new().map_err(|e| format!("failed to initialize clipboard: {e}"))?;
    clipboard
        .set_text(message.to_string())
        .map_err(|e| format!("failed to copy to clipboard: {e}"))?;
    writeln!(
        std::io::stdout(),
        "the commit message has been copied to clipboard."
    )?;
    Ok(())
}

/// Confirm with the user (unless `--yes`) and create the commit.
fn run_commit_flow(repository: &Repository, message: &GitMessage, yes: bool) -> utils::Result<()> {
    trace!("commit option is enabled, will commit the changes directly to the repository");

    let should_commit = yes || {
        cliclack::intro(format!("{PKG_NAME} v{PKG_VERSION}"))?;
        cliclack::confirm("Are you sure to commit with generated message below?").interact()?
    };

    if should_commit {
        match repository.commit(message) {
            Ok(oid) => cliclack::note("Commit successful, last commit ID:", oid)?,
            Err(e) => cliclack::note("Commit failed", e)?,
        }
    }
    cliclack::outro("Bye~")?;
    Ok(())
}

/// Initialize the global tracing subscriber.
///
/// Default level is WARN so that fallback warnings (missing git user.email,
/// invalid timeout values, …) reach the user. `--verbose` upgrades to TRACE.
#[inline]
fn init_logging(verbose: bool) {
    let level = if verbose { Level::TRACE } else { Level::WARN };
    let _ = tracing_subscriber::fmt()
        .with_max_level(level)
        .without_time()
        .with_target(false)
        .try_init();

    if verbose {
        trace!("verbose mode enabled (TRACE-level logging)");
    }
}

/// Verify the configured model is reachable.
async fn check_model_availability(client: &OpenAI, model_name: &str) -> utils::Result<()> {
    client.check_model(model_name).await?;
    println!("the model name `{model_name}` is available, {PKG_NAME} is ready for use!");
    Ok(())
}

/// Build the chat request and call the OpenAI API, returning the raw response.
async fn request_completion(
    client: &OpenAI,
    model_name: &str,
    logs: &[String],
    diffs: &[String],
) -> utils::Result<String> {
    let content = OpenAI::prompt(logs, diffs)?;
    let messages = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(SYSTEM_PROMPT)
            .build()?
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content(content)
            .build()?
            .into(),
    ];
    Ok(client.chat(model_name, messages).await?)
}
