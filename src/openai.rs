/*
 * Copyright (c) 2025 Hangzhou Guanwaii Technology Co,.Ltd.
 *
 * This source code is licensed under the MIT License,
 * which is located in the LICENSE file in the source tree's root directory.
 *
 * File: openai.rs
 * Author: mingcheng (mingcheng@apache.org)
 * File Created: 2025-03-01 21:55:58
 *
 * Modified By: mingcheng (mingcheng@apache.org)
 * Last Modified: 2025-03-03 17:50:02
 */

use askama::Template;
use async_openai::config::OPENAI_API_BASE;
use async_openai::{
    config::OpenAIConfig,
    types::{ChatCompletionRequestMessage, CreateChatCompletionRequestArgs},
    Client,
};
use log::trace;
use reqwest::{ClientBuilder, Proxy};
use std::env;
use std::error::Error;
use tracing::debug;

#[derive(Template)]
#[template(path = "user.txt")]
struct PromptTemplate<'a> {
    logs: &'a str,
    diffs: &'a str,
}
pub struct OpenAI {
    client: Client<OpenAIConfig>,
}

impl Default for OpenAI {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenAI {
    pub fn new() -> Self {
        let ai_config = OpenAIConfig::new()
            .with_api_key(env::var("OPENAI_API_TOKEN").unwrap_or_else(|_| String::from("")))
            .with_api_base(
                env::var("OPENAI_API_BASE").unwrap_or_else(|_| String::from(OPENAI_API_BASE)),
            );
        let proxy_addr = env::var("OPENAI_APT_PROXY").unwrap_or_else(|_| String::from(""));

        let mut client = Client::with_config(ai_config);
        if !proxy_addr.is_empty() {
            trace!("Using proxy: {}", proxy_addr);
            client = client.with_http_client({
                let proxy = Proxy::all(proxy_addr).unwrap();
                ClientBuilder::new().proxy(proxy).build().unwrap()
            })
        };

        OpenAI { client }
    }

    pub async fn check(&self) -> Result<(), Box<dyn Error>> {
        match self.client.models().list().await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn chat(
        &self,
        model_name: &str,
        message: Vec<ChatCompletionRequestMessage>,
    ) -> Result<String, Box<dyn Error>> {
        let request = CreateChatCompletionRequestArgs::default()
            .model(model_name)
            .messages(message)
            .build()?;

        let response = self.client.chat().create(request).await?;

        let mut result = vec![];
        response.choices.iter().for_each(|choice| {
            result.push(choice.message.content.as_ref().unwrap().to_string());
        });

        if let Option::Some(usage) = response.usage {
            debug!(
                "Usage: completion_tokens: {}, prompt_tokens: {}, total_tokens: {}",
                usage.completion_tokens, usage.prompt_tokens, usage.total_tokens
            );
        }

        Ok(result.join("\n"))
    }

    pub fn prompt(logs: &[String], diff: &[String]) -> Result<String, Box<dyn Error>> {
        let template = PromptTemplate {
            logs: &logs.join("\n"),
            diffs: &diff.join("\n"),
        };

        match template.render() {
            Ok(content) => Ok(content),
            Err(e) => Err(Box::new(e)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::git::Git;
    const REPO_PATH: &str = "";

    #[test]
    fn test_prompt() {
        let git = Git::new(REPO_PATH).unwrap();
        let diffs = git.get_diff();
        assert!(diffs.is_ok());

        let logs = git.get_logs(5);
        assert!(logs.is_ok());

        let diff_content = diffs.unwrap();
        assert!(diff_content.len() > 0);

        let logs_content = logs.unwrap();
        assert!(logs_content.len() > 0);

        let result = OpenAI::prompt(&logs_content, &diff_content).unwrap();

        print!("{}", result);
    }
}
