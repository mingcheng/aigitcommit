/*
 * Copyright (c) 2025 Hangzhou Guanwaii Technology Co,.Ltd.
 *
 * This source code is licensed under the MIT License,
 * which is located in the LICENSE file in the source tree's root directory.
 *
 * File: main.rs
 * Author: mingcheng (mingcheng@apache.org)
 * File Created: 2025-03-01 17:17:30
 *
 * Modified By: mingcheng (mingcheng@apache.org)
 * Last Modified: 2025-07-11 17:43:19
 */

use aigitcommit::cli::Cli;
use aigitcommit::git::Git;
use aigitcommit::openai;
use aigitcommit::openai::OpenAI;
use arboard::Clipboard;
use async_openai::error::OpenAIError;
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};
use clap::Parser;
use dialoguer::Confirm;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::{env, fs};
use tracing::{debug, trace, Level};

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

    // Check if the specified path is a valid directory
    let repo_dir = fs::canonicalize(&cli.repo_path)?;

    // Check if the directory is empty
    if !repo_dir.is_dir() {
        return Err("the specified path is not a directory".into());
    }

    trace!("specified repository directory: {:?}", repo_dir);
    // Check if the directory is a valid git repository
    let repository = Git::new(repo_dir.to_str().unwrap_or("."))?;

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

    // Instantiate OpenAI client, ready to send requests to the OpenAI API
    let client = openai::OpenAI::new();

    // Check if the OpenAI request is valid, if not, return error
    // if client.check().await.is_err() {
    //     return Err(
    //         "OpenAI API check with error, please check your API key or configuration".into(),
    //     );
    // };

    // Generate the prompt which will be sent to OpenAI API
    let content = OpenAI::prompt(&logs, &diffs)?;

    // Get the specified model name from environment variable, default to "gpt-4"
    let model_name = env::var("OPENAI_MODEL_NAME").unwrap_or_else(|_| String::from("gpt-4"));

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
    let mut result = match client.chat(&model_name.to_string(), messages).await {
        Ok(s) => s,
        Err(e) => {
            let message = match e {
                OpenAIError::Reqwest(_) | OpenAIError::StreamError(_) => {
                    "network request error".to_string()
                }
                OpenAIError::JSONDeserialize(_err) => "json deserialization error".to_string(),
                OpenAIError::InvalidArgument(_) => "invalid argument".to_string(),
                OpenAIError::FileSaveError(_) | OpenAIError::FileReadError(_) => {
                    "io error".to_string()
                }
                OpenAIError::ApiError(e) => format!("api error {e:?}"),
            };

            return Err(message.into());
        }
    };

    // Detect auto signoff from environment variable
    let need_signoff = cli.signoff
        || env::var("GIT_AUTO_SIGNOFF")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

    // If the --signoff option is enabled, add signoff to the commit message
    if need_signoff {
        trace!("signoff option is enabled, will add signoff to the commit message");
        let (author_name, author_email) = (
            repository.get_author_name()?,
            repository.get_author_email()?,
        );

        // Add signoff to the commit message
        let signoff = format!("\n\nSigned-off-by: {author_name} <{author_email}>");
        result.push_str(&signoff);
    }

    // Detect auto signoff from environment variable
    let need_signoff = cli.signoff
        || env::var("GIT_AUTO_SIGNOFF")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

    // If the --signoff option is enabled, add signoff to the commit message
    if need_signoff {
        trace!("signoff option is enabled, will add signoff to the commit message");
        let (author_name, author_email) = (
            repository.get_author_name()?,
            repository.get_author_email()?,
        );

        // Add signoff to the commit message
        let signoff = format!("\n\nSigned-off-by: {author_name} <{author_email}>");
        result.push_str(&signoff);
    }

    trace!("write to stdout, and finish the process");
    writeln!(std::io::stdout(), "{result}")?;

    // Copy the commit message to clipboard if the --copy option is enabled
    if cli.copy {
        let mut clipboard = Clipboard::new()?;
        clipboard.set_text(&result)?;
        writeln!(
            std::io::stdout(),
            "the commit message has been copied to clipboard."
        )?;
    }

    // directly commit the changes to the repository if the --commit option is enabled
    if cli.commit {
        trace!("commit option is enabled, will commit the changes to the repository");
        let mut confirm = Confirm::new();
        confirm
            .with_prompt("do you want to commit the changes with the generated commit message?")
            .default(false);

        // Prompt the user for confirmation if --yes option is not enabled
        if cli.yes || confirm.interact()? {
            match repository.commit(&result) {
                Ok(_) => {
                    writeln!(std::io::stdout(), "commit successful!")?;
                }
                Err(e) => {
                    writeln!(std::io::stderr(), "commit failed: {e}")?;
                }
            }
        }
    }

    // If the --save option is enabled, save the commit message to a file
    if !cli.save.is_empty() {
        trace!("save option is enabled, will save the commit message to a file");
        let save_path = &cli.save;
        debug!("the save file path is {:?}", &save_path);

        let mut file = File::create(save_path)?;
        file.write_all(result.as_bytes())?;
        file.flush()?;

        writeln!(std::io::stdout(), "commit message saved to {save_path}")?;
    }

    Ok(())
}
