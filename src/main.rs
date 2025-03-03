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
 * Last Modified: 2025-03-03 10:35:45
 */

use aigitcommit::openai::OpenAI;
use aigitcommit::{git, openai};
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};
use std::env;
use std::error::Error;
use std::io::Write;
use tracing::{debug, error, trace, Level};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .without_time()
        .with_target(false)
        .init();

    let repo_dir = {
        let args: Vec<String> = env::args().collect();
        if args.len() > 1 {
            args[1].clone()
        } else {
            env::current_dir()?.to_string_lossy().to_string()
        }
    };

    if repo_dir.is_empty() {
        error!("No directory specified");
        return Err("No directory specified".into());
    }
    trace!("Specified repository directory: {}", repo_dir);

    // Check if there is at least one argument (excluding the program name)

    let repository = git::Git::new(&repo_dir)?;

    let diffs = repository.get_diff()?;
    debug!("Got diff size is {}", diffs.len());

    let logs = repository.get_logs(5)?;
    debug!("Got logs size is {}", logs.len());

    let client = openai::OpenAI::new();

    if client.check().await.is_err() {
        error!("OpenAI API check with error, please check your API key or configuration");
        return Err(
            "OpenAI API check with error, please check your API key or configuration".into(),
        );
    };

    let content = OpenAI::prompt(&logs, &diffs)?;

    let model_name = env::var("OPENAI_MODEL_NAME").unwrap_or_else(|_| String::from("gpt-4"));
    let messages = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(
                r#"
               You act as an informed senior in software development and
               You must speak English as your primary language.
               Meanwhile, you have contributed to the open-source community for many years."#,
            )
            .build()?
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content(content)
            .build()?
            .into(),
    ];

    let result = client.chat(&model_name.to_string(), messages).await?;

    trace!("write to stdout, and finish the process");
    writeln!(std::io::stdout(), "{}", result)?;

    Ok(())
}
