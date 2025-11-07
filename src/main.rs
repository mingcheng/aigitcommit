/*!
 * Copyright (c) 2025 Hangzhou Guanwaii Technology Co., Ltd.
 *
 * This source code is licensed under the MIT License,
 * which is located in the LICENSE file in the source tree's root directory.
 *
 * File: main.rs
 * Author: mingcheng <mingcheng@apache.org>
 * File Created: 2025-03-01 17:17:30
 *
 * Modified By: mingcheng <mingcheng@apache.org>
 * Last Modified: 2025-11-07 14:29:29
 */

use aigitcommit::built_info::{PKG_NAME, PKG_VERSION};
use aigitcommit::cli::Cli;
use aigitcommit::git::message::GitMessage;
use aigitcommit::git::repository::Repository;
use aigitcommit::openai;
use aigitcommit::openai::OpenAI;
use arboard::Clipboard;
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};
use clap::Parser;
use std::error::Error;
use std::fs;
use std::io::Write;
use tracing::{Level, debug, error, info, trace};

use aigitcommit::utils::{
    OutputFormat, check_env_variables, env, format_openai_error, save_to_file, should_signoff,
};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(Level::TRACE)
            .without_time()
            .with_target(false)
            .init();

        trace!(
            "verbose mode enabled, set the log level to TRACE. It will makes a little bit noise."
        );
    }

    // Get the specified model name from environment variable, default to "gpt-5"
    let model_name = env::get("OPENAI_MODEL_NAME", "gpt-5");

    // Instantiate OpenAI client, ready to send requests to the OpenAI API
    let client = openai::OpenAI::new();

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

        match client.check_model(&model_name).await {
            Ok(()) => {
                println!(
                    "the model name `{}` is available, {} is ready for use!",
                    model_name, PKG_NAME
                );
            }
            Err(e) => {
                return Err(format!("the model name `{model_name}` is not available: {e}").into());
            }
        }

        return Ok(());
    }

    // Initialize repository
    let repo_dir = fs::canonicalize(&cli.repo_path)?;
    if !repo_dir.is_dir() {
        return Err("the specified path is not a directory".into());
    }

    trace!("specified repository directory: {:?}", repo_dir);
    let repository = Repository::new(repo_dir.to_str().unwrap_or("."))?;

    // Get the diff and logs from the repository
    let diffs = repository.get_diff()?;
    debug!("got diff size is {}", diffs.len());
    if diffs.is_empty() {
        return Err("no diff found".into());
    }

    // Get the last 5 commit logs
    // if the repository has less than 5 commits, it will return all logs
    let logs = repository.get_logs(5)?;
    debug!("got logs size is {}", logs.len());

    // If git commit log is empty, return error
    if logs.is_empty() {
        return Err("no commit logs found".into());
    }

    // Generate the prompt which will be sent to OpenAI API
    let content = OpenAI::prompt(&logs, &diffs)?;

    // Load the system prompt from the template file
    let system_prompt = include_str!("../templates/system.txt");

    // The request contains the system message and user message
    let messages = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(system_prompt)
            .build()?
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content(content)
            .build()?
            .into(),
    ];

    // Send the request to OpenAI API and get the response
    let result = match client.chat(&model_name, messages).await {
        Ok(s) => s,
        Err(e) => {
            let message = format_openai_error(e);
            return Err(message.into());
        }
    };

    let (title, content) = result
        .split_once("\n\n")
        .ok_or("Invalid response format: expected title and content separated by double newline")?;

    // Detect auto signoff from environment variable or CLI flag
    let need_signoff = should_signoff(&repository, cli.signoff);

    let message: GitMessage = GitMessage::new(&repository, title, content, need_signoff)?;

    // Decide the output format based on the command line arguments
    let output_format = OutputFormat::detect(cli.json, cli.no_table);
    output_format.write(&message)?;

    // Copy the commit message to clipboard if the --copy option is enabled
    if cli.copy_to_clipboard {
        let mut clipboard = Clipboard::new()?;
        clipboard.set_text(message.to_string())?;
        writeln!(
            std::io::stdout(),
            "the commit message has been copied to clipboard."
        )?;
    }

    // directly commit the changes to the repository if the --commit option is enabled
    if cli.commit {
        trace!("commit option is enabled, will commit the changes directly to the repository");

        if cli.yes || {
            cliclack::intro(format!("{PKG_NAME} v{PKG_VERSION}"))?;
            cliclack::confirm("Are you sure to commit with generated message below?").interact()?
        } {
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
        match save_to_file(&cli.save, &message) {
            Ok(f) => {
                info!("commit message saved to file: {:?}", f);
            }
            Err(e) => {
                error!("failed to save commit message to file: {}", e);
            }
        }
    }

    Ok(())
}
