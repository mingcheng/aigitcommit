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
 * Last Modified: 2025-03-05 00:12:26
 */

use aigitcommit::cli::Cli;
use aigitcommit::openai::OpenAI;
use aigitcommit::{git, openai};
use arboard::Clipboard;
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};
use clap::Parser;
use dialoguer::Confirm;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::{env, fs};
use tracing::{Level, debug, trace};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(if cli.verbose {
            trace!("Verbose mode enabled, set the log level to TRACE. It will makes a little bit noise.");
            Level::TRACE
        } else {
            debug!("Verbose mode disabled, set the default log level to WARN");
            Level::WARN
        })
        .without_time()
        .with_target(false)
        .init();

    // Check if the specified path is a valid directory
    let repo_dir = fs::canonicalize(&cli.repo_path)?;

    // Check if the directory is empty
    if !repo_dir.is_dir() {
        return Err("The specified path is not a directory".into());
    }

    trace!("Specified repository directory: {:?}", repo_dir);
    // Check if the directory is a valid git repository
    let repository = git::Git::new(repo_dir.to_str().unwrap_or("."))?;

    // Get the diff and logs from the repository
    let diffs = repository.get_diff()?;
    debug!("Got diff size is {}", diffs.len());
    if diffs.is_empty() {
        return Err("No diff found".into());
    }

    // Get the last 5 commit logs
    // if the repository has less than 5 commits, it will return all logs
    let logs = repository.get_logs(5)?;
    debug!("Got logs size is {}", logs.len());

    // If git commit log is empty, return error
    if logs.is_empty() {
        return Err("No commit logs found".into());
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
    let result = client.chat(&model_name.to_string(), messages).await?;

    trace!("write to stdout, and finish the process");
    writeln!(std::io::stdout(), "{}", result)?;

    // Copy the commit message to clipboard if the --copy option is enabled
    if cli.copy {
        let mut clipboard = Clipboard::new()?;
        clipboard.set_text(&result)?;
        writeln!(
            std::io::stdout(),
            "The commit message has been copied to clipboard."
        )?;
    }

    // directly commit the changes to the repository if the --commit option is enabled
    if cli.commit {
        trace!("Commit option is enabled, will commit the changes to the repository");
        let mut confirm = Confirm::new();
        confirm
            .with_prompt("Do you want to commit the changes with the generated commit message?")
            .default(false);

        // Prompt the user for confirmation if --yes option is not enabled
        if cli.yes || confirm.interact()? {
            match repository.commit(&result) {
                Ok(_) => {
                    writeln!(std::io::stdout(), "Commit successful!")?;
                }
                Err(e) => {
                    writeln!(std::io::stderr(), "Commit failed: {}", e)?;
                }
            }
        }
    }

    // If the --save option is enabled, save the commit message to a file
    if !cli.save.is_empty() {
        trace!("Save option is enabled, will save the commit message to a file");
        let save_path = &cli.save;
        debug!("The save file path is {:?}", &save_path);

        let mut file = File::create(save_path)?;
        file.write_all(result.as_bytes())?;
        file.flush()?;

        writeln!(std::io::stdout(), "Commit message saved to {}", &save_path)?;
    }

    Ok(())
}
