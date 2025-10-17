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
 * Last Modified: 2025-09-26 15:45:37
 */

use aigitcommit::cli::{Cli, print_table};
use aigitcommit::git::message::GitMessage;
use aigitcommit::git::repository::Repository;
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
use tracing::{Level, debug, trace};
mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

/// The output format for the commit message
#[derive(Debug)]
enum OutPutFormat {
    Stdout,
    Table,
    Json,
}

/// Detect the output format based on the command line arguments
fn detect_output_format(cli: &Cli) -> OutPutFormat {
    if cli.json {
        return OutPutFormat::Json;
    } else if cli.no_table {
        return OutPutFormat::Stdout;
    }

    OutPutFormat::Table
}

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

    // Get the specified model name from environment variable, default to "gpt-4"
    let model_name = env::var("OPENAI_MODEL_NAME").unwrap_or_else(|_| String::from("gpt-5"));

    // Instantiate OpenAI client, ready to send requests to the OpenAI API
    let client = openai::OpenAI::new();

    // Check if the environment variables are set and print the configured values
    if cli.check_env {
        fn check_and_print_env(var_name: &str) {
            match env::var(var_name) {
                Ok(value) => {
                    debug!("{} is set to {}", var_name, value);
                    // Print the value of the environment variable
                    println!("{:20}\t{}", var_name, value);
                }
                Err(_) => {
                    debug!("{} is not set", var_name);
                }
            }
        }

        trace!("check env option is enabled, will check the OpenAI API key and model name");
        debug!("the model name is `{}`", &model_name);

        [
            "OPENAI_API_BASE",
            "OPENAI_API_TOKEN",
            "OPENAI_MODEL_NAME",
            "OPENAI_API_PROXY",
            "OPENAI_API_TIMEOUT",
            "OPENAI_API_MAX_TOKENS",
            "GIT_AUTO_SIGNOFF",
        ]
        .iter()
        .for_each(|v| check_and_print_env(v));

        return Ok(());
    }

    // Check if the model name is valid
    if cli.check_model {
        trace!("check option is enabled, will check the OpenAI API key and model name");
        debug!("the model name is `{}`", &model_name);

        match client.check_model(&model_name).await {
            Ok(()) => {
                println!(
                    "the model name `{}` is available, {} is ready for use!",
                    model_name,
                    built_info::PKG_NAME
                );
            }
            Err(e) => {
                return Err(format!("the model name `{model_name}` is not available: {e}").into());
            }
        }

        return Ok(());
    }

    // Check if the specified path is a valid directory
    let repo_dir = fs::canonicalize(&cli.repo_path)?;

    // Check if the directory is empty
    if !repo_dir.is_dir() {
        return Err("the specified path is not a directory".into());
    }

    trace!("specified repository directory: {:?}", repo_dir);
    // Check if the directory is a valid git repository
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
    let result = match client.chat(&model_name.to_string(), messages).await {
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

    let (title, content) = result.split_once("\n\n").unwrap();

    // Detect auto signoff from environment variable
    let need_signoff = cli.signoff
        || env::var("GIT_AUTO_SIGNOFF")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

    let message: GitMessage = GitMessage::new(&repository, title, content, need_signoff)?;

    // Decide the output format based on the command line arguments
    match detect_output_format(&cli) {
        OutPutFormat::Stdout => {
            // Write the commit message to stdout
            trace!("write to stdout, and finish the process");
            writeln!(std::io::stdout(), "{}", message)?;
        }
        OutPutFormat::Json => {
            // Print the commit message in JSON format
            let json = serde_json::to_string_pretty(&message)?;
            writeln!(std::io::stdout(), "{}", json)?;
        }
        OutPutFormat::Table => {
            // Default print message in table
            print_table(&message.title, &message.content);
        }
    };

    // Copy the commit message to clipboard if the --copy option is enabled
    if cli.copy_to_clipboard {
        let mut clipboard = Clipboard::new()?;
        clipboard.set_text(format!("{}", &message))?;
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
            match repository.commit(&message) {
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
