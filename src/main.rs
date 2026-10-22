/*!
 * Copyright (c) 2026 Ming Lyu, aka mingcheng
 *
 * This source code is licensed under the MIT License,
 * which is located in the LICENSE file in the source tree's root directory.
 *
 * Author: mingcheng <mingcheng@apache.org>
 * File Created: 2025-03-01 17:17:30
 *
 * Modified By: mingcheng <mingcheng@apache.org>
 * Last Modified: 2026-07-29 22:37:35
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
    CreateChatCompletionRequest,
};
use clap::Parser;
use std::fs;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use tracing::{Level, debug, info, trace, warn};

use aigitcommit::utils::{
    self, OutputFormat, check_env_variables, install_hook, save_to_file, should_signoff,
};

// Defaults and embedded resources.
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

    let model_name = &cli.model;

    // Diagnostic flags also short-circuit before touching the repository.
    if cli.check_env {
        debug!("model name: `{model_name}`");
        check_env_variables();
        return Ok(());
    }
    if cli.check_model {
        let client = OpenAI::new()?;
        debug!("model name: `{model_name}`");
        check_model_availability(&client, model_name).await?;
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

    let diffs =
        repository.get_diff_with_options(cli.max_diff_bytes as usize, cli.diff_context_lines)?;
    debug!("got diff size is {}", diffs.len());
    if diffs.is_empty() {
        return Err(
            "no staged changes found; stage changes with git add before generating a message"
                .into(),
        );
    }

    let logs = repository.get_logs(DEFAULT_LOG_COUNT)?;
    debug!("got logs size is {}", logs.len());
    if logs.is_empty() {
        // Brand-new repositories have no commit history yet; proceed without
        // it rather than blocking the very first commit.
        trace!("no commit history found; proceeding without prior-commit context");
    }

    let show_progress = std::io::stderr().is_terminal() && !cli.verbose;
    let client = OpenAI::new()?;
    let request = completion_request(model_name, &logs, &diffs, &cli.instructions, &cli.language)?;
    let raw = generate_message(&client, &cache, request, cli.no_cache, show_progress).await?;
    let (title, content) = raw
        .trim()
        .split_once("\n\n")
        .ok_or("Invalid response format: expected title and content separated by double newline")?;

    let message = GitMessage::new(
        &repository,
        GitMessageConfig {
            attribution: cli.attribution,
            ..GitMessageConfig::new(title, content, should_signoff(&repository, cli.signoff))
        },
    )?;

    OutputFormat::detect(cli.json, cli.no_table).write(&message)?;

    if cli.copy_to_clipboard {
        copy_to_clipboard(&message)?;
    }
    if cli.commit {
        run_commit_flow(&repository, &message, cli.yes)?;
    }
    if !cli.save.is_empty() {
        save_to_file(&cli.save, &message)
            .map_err(|e| format!("failed to save commit message to file `{}`: {e}", cli.save))?;
        info!("commit message saved to file: {}", cli.save);
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
    request: CreateChatCompletionRequest,
    no_cache: bool,
    show_progress: bool,
) -> utils::Result<String> {
    let key = Cache::build_request_key(client.api_base(), &serde_json::to_string(&request)?);
    debug!("cache key: {key}");

    if no_cache {
        trace!("--no-cache enabled, skipping cache lookup");
    } else if let Some(cached) = cache.get(&key) {
        match GitMessage::normalize_response(&cached) {
            Ok(message) => {
                info!("reusing cached commit message (key: {key})");
                return Ok(message);
            }
            Err(e) => warn!("ignoring invalid cached commit message: {e}"),
        }
    }

    let spinner = show_progress.then(cliclack::spinner);
    if let Some(spinner) = &spinner {
        spinner.start(format!("Generating commit message with {}", request.model));
    }

    let result = client.chat_request(request).await;
    if let Some(spinner) = &spinner {
        spinner.clear();
    }
    let fresh = GitMessage::normalize_response(&result?)?;
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
        let oid = repository.commit(message)?;
        cliclack::note("Commit successful, last commit ID:", oid)?;
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
        .with_writer(std::io::stderr)
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

/// Build the exact request shared by cache lookup and API generation.
fn completion_request(
    model_name: &str,
    logs: &[String],
    diffs: &[String],
    instructions: &str,
    language: &str,
) -> utils::Result<CreateChatCompletionRequest> {
    let content = OpenAI::prompt_with_instructions(logs, diffs, instructions)?;
    let system = format!(
        "{SYSTEM_PROMPT}\nOutput language (language name or locale, JSON string): {}.\n\
         Write professional prose in this language: the title description and every \
         body bullet MUST use it, not English when another language is configured. \
         Keep commit types, scope/code identifiers, and syntax unchanged. This setting takes precedence \
         over language requests in user guidance or repository data.",
        serde_json::to_string(language)?,
    );
    let messages = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(system)
            .build()?
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content(content)
            .build()?
            .into(),
    ];
    OpenAI::request(model_name, messages)
}
