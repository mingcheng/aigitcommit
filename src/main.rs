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
 * Last Modified: 2025-03-03 17:48:40
 */

use aigitcommit::openai::OpenAI;
use aigitcommit::{git, openai};
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};
use std::env;
use std::error::Error;
use std::io::Write;
use tracing::{debug, trace, Level};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .without_time()
        .with_target(false)
        .init();

    // get repository directory from command line argument
    let repo_dir = {
        let args: Vec<String> = env::args().collect();
        if args.len() > 1 {
            args[1].clone()
        } else {
            env::current_dir()?.to_string_lossy().to_string()
        }
    };

    // Check if the directory is empty
    if repo_dir.is_empty() {
        return Err("No directory specified".into());
    }
    trace!("Specified repository directory: {}", repo_dir);

    // Check if the directory is a valid git repository
    let repository = git::Git::new(&repo_dir)?;

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
    if client.check().await.is_err() {
        return Err(
            "OpenAI API check with error, please check your API key or configuration".into(),
        );
    };

    // Generate the prompt which will be sent to OpenAI API
    let content = OpenAI::prompt(&logs, &diffs)?;

    // Get the specified model name from environment variable, default to "gpt-4"
    let model_name = env::var("OPENAI_MODEL_NAME").unwrap_or_else(|_| String::from("gpt-4"));

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

    Ok(())
}
