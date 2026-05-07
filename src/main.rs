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
use aigitcommit::git::message::GitMessage;
use aigitcommit::git::repository::Repository;
use aigitcommit::openai::OpenAI;
use arboard::Clipboard;
use async_openai::types::chat::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};
use clap::Parser;
use std::fs;
use std::io::Write;
use std::path::Path;
use tracing::{Level, debug, error, info, trace};

use aigitcommit::utils::{
    self, OutputFormat, check_env_variables, env, install_hook, save_to_file, should_signoff,
};

// Constants for better performance and maintainability
const DEFAULT_MODEL: &str = "gpt-5";
const DEFAULT_LOG_COUNT: usize = 5;
const SYSTEM_PROMPT: &str = include_str!("../templates/system.txt");

// Constant for git hook installation
const HOOK_NAME: &str = "prepare-commit-msg";
const HOOK_CONTENT: &str = include_str!("../hooks/prepare-commit-msg");

#[tokio::main]
async fn main() -> utils::Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Initialize logging
    init_logging(cli.verbose);

    // Handle subcommands early and exit
    if let Some(Command::InstallHook { repo_path }) = &cli.command {
        trace!("install-hook subcommand invoked");
        install_hook(repo_path, HOOK_NAME, HOOK_CONTENT)?;
        println!("git hook `{}` has been installed successfully.", HOOK_NAME);
        return Ok(());
    }

    // Get the specified model name from environment variable, default constant
    let model_name = env::get("OPENAI_MODEL_NAME", DEFAULT_MODEL);

    // Instantiate OpenAI client, ready to send requests to the OpenAI API
    let client = OpenAI::new();

    // Check if the environment variables are set and print the configured values
    if cli.check_env {
        trace!("check env option is enabled");
        debug!("model name: `{}`", &model_name);
        check_env_variables();
        return Ok(());
    }

    // Check if the model name is valid
    if cli.check_model {
        trace!("check model option is enabled");
        debug!("model name: `{}`", &model_name);
        check_model_availability(&client, &model_name).await?;
        return Ok(());
    }

    // Initialize repository
    let repo_path = Path::new(&cli.repo_path);
    let repo_dir = fs::canonicalize(repo_path)
        .map_err(|e| format!("failed to resolve repository path: {e}"))?;

    if !repo_dir.is_dir() {
        return Err("the specified path is not a directory".into());
    }

    trace!("specified repository directory: {:?}", repo_dir);
    let repository = Repository::new(
        repo_dir
            .to_str()
            .ok_or("invalid UTF-8 in repository path")?,
    )?;

    // Initialize the local cache rooted under the repository's .git directory
    let cache = Cache::new(repository.git_dir());

    // Handle --clear-cache early: wipe the cache and exit before doing any work
    if cli.clear_cache {
        match cache.clear() {
            Ok(n) => {
                info!("cleared {} cache entries", n);
                writeln!(std::io::stdout(), "cleared {n} cache entries.")?;
            }
            Err(e) => return Err(format!("failed to clear cache: {e}").into()),
        }
        return Ok(());
    }

    // Get the diff and logs from the repository
    let diffs = repository.get_diff()?;
    debug!("got diff size is {}", diffs.len());

    if diffs.is_empty() {
        return Err("no changes found in the repository".into());
    }

    // Get the last N commit logs
    // if the repository has less than N commits, it will return all logs
    let logs = repository.get_logs(DEFAULT_LOG_COUNT)?;
    debug!("got logs size is {}", logs.len());

    // If git commit log is empty, return error
    if logs.is_empty() {
        return Err("no commit history found in the repository".into());
    }

    // Compute a stable cache key from the inputs that influence the API request.
    // When nothing has changed (same diff, same recent logs, same model & prompt)
    // we can reuse the previously generated message and skip the API call.
    let cache_key = Cache::build_key(&model_name, SYSTEM_PROMPT, &diffs, &logs);
    debug!("cache key: {}", cache_key);

    let result = if !cli.no_cache {
        if let Some(cached) = cache.get(&cache_key) {
            info!("reusing cached commit message (key: {})", cache_key);
            cached
        } else {
            let fresh = request_completion(&client, &model_name, &logs, &diffs).await?;
            cache.put(&cache_key, &fresh);
            fresh
        }
    } else {
        trace!("--no-cache enabled, skipping cache lookup");
        request_completion(&client, &model_name, &logs, &diffs).await?
    };

    let (title, content) = result
        .split_once("\n\n")
        .ok_or("Invalid response format: expected title and content separated by double newline")?;

    // Detect auto signoff from environment variable or CLI flag
    let need_signoff = should_signoff(&repository, cli.signoff);

    let message = GitMessage::new(&repository, title, content, need_signoff)?;

    // Decide the output format based on the command line arguments
    let output_format = OutputFormat::detect(cli.json, cli.no_table);
    output_format.write(&message)?;

    // Copy the commit message to clipboard if the --copy option is enabled
    if cli.copy_to_clipboard {
        let mut clipboard =
            Clipboard::new().map_err(|e| format!("failed to initialize clipboard: {e}"))?;
        clipboard
            .set_text(message.to_string())
            .map_err(|e| format!("failed to copy to clipboard: {e}"))?;
        writeln!(
            std::io::stdout(),
            "the commit message has been copied to clipboard."
        )?;
    }

    // directly commit the changes to the repository if the --commit option is enabled
    if cli.commit {
        trace!("commit option is enabled, will commit the changes directly to the repository");

        let should_commit = cli.yes || {
            cliclack::intro(format!("{PKG_NAME} v{PKG_VERSION}"))?;
            cliclack::confirm("Are you sure to commit with generated message below?").interact()?
        };

        if should_commit {
            match repository.commit(&message) {
                Ok(oid) => {
                    cliclack::note("Commit successful, last commit ID:", oid)?;
                }
                Err(e) => {
                    cliclack::note("Commit failed", e)?;
                }
            }
        }

        cliclack::outro("Bye~")?;
    }

    // If the --save option is enabled, save the commit message to a file
    if !cli.save.is_empty() {
        trace!("save option is enabled, will save the commit message to a file");

        // Save the commit message to the specified file
        save_to_file(&cli.save, &message)
            .map(|f| info!("commit message saved to file: {:?}", f))
            .unwrap_or_else(|e| error!("failed to save commit message to file: {}", e));
    }

    Ok(())
}

/// Initialize logging based on verbosity level
#[inline]
fn init_logging(verbose: bool) {
    if verbose {
        tracing_subscriber::fmt()
            .with_max_level(Level::TRACE)
            .without_time()
            .with_target(false)
            .init();

        trace!(
            "verbose mode enabled, set the log level to TRACE. It will makes a little bit noise."
        );
    }
}

/// Check if the model is available
async fn check_model_availability(client: &OpenAI, model_name: &str) -> utils::Result<()> {
    client.check_model(model_name).await?;
    println!(
        "the model name `{}` is available, {} is ready for use!",
        model_name, PKG_NAME
    );
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
