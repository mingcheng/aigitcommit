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
 * Last Modified: 2025-07-01 07:47:01
 */

use askama::Template;
use async_openai::config::OPENAI_API_BASE;
use async_openai::error::OpenAIError;
use async_openai::{
    config::OpenAIConfig,
    types::{ChatCompletionRequestMessage, CreateChatCompletionRequestArgs},
    Client,
};
use log::trace;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{ClientBuilder, Proxy};
use std::env;
use std::error::Error;
use std::time::Duration;
use tracing::debug;

use crate::cli;

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
    /// Create a new OpenAI client instance.
    /// This function sets up the OpenAI client with the API key, base URL, and optional proxy settings.
    pub fn new() -> Self {
        // Set up OpenAI client configuration
        let ai_config = OpenAIConfig::new()
            .with_api_key(env::var("OPENAI_API_TOKEN").unwrap_or_else(|_| String::from("")))
            .with_api_base(
                env::var("OPENAI_API_BASE").unwrap_or_else(|_| String::from(OPENAI_API_BASE)),
            )
            .with_org_id(cli::CMD);

        // Set up HTTP client builder with default headers
        let mut http_client_builder = ClientBuilder::new().user_agent(cli::CMD).default_headers({
            let mut headers = HeaderMap::new();
            headers.insert("HTTP-Referer", HeaderValue::from_static(cli::CMD_ABOUT_URL));
            headers.insert("X-Title", HeaderValue::from_static(cli::CMD));
            headers
        });

        // Set up proxy if specified
        let proxy_addr: String = env::var("OPENAI_API_PROXY").unwrap_or_else(|_| String::from(""));
        if !proxy_addr.is_empty() {
            trace!("Using proxy: {proxy_addr}");
            http_client_builder = http_client_builder.proxy(Proxy::all(proxy_addr).unwrap());
        }

        let request_timeout =
            env::var("OPENAI_REQUEST_TIMEOUT").unwrap_or_else(|_| String::from(""));
        if !request_timeout.is_empty() {
            if let Ok(timeout) = request_timeout.parse::<u64>() {
                trace!("Setting request timeout to: {request_timeout}ms");
                http_client_builder = http_client_builder.timeout(Duration::from_millis(timeout));
            }
        }

        // Set up timeout and build the HTTP client
        let http_client = http_client_builder.build().unwrap();

        let client = Client::with_config(ai_config).with_http_client(http_client);
        OpenAI { client }
    }

    #[deprecated]
    /// Check if the OpenAI API is reachable.
    pub async fn check(&self) -> Result<(), Box<dyn Error>> {
        match self.client.models().list().await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    /// Send a chat message to the OpenAI API and return the response.
    pub async fn chat(
        &self,
        model_name: &str,
        message: Vec<ChatCompletionRequestMessage>,
    ) -> Result<String, OpenAIError> {
        let request = CreateChatCompletionRequestArgs::default()
            .model(model_name)
            .messages(message)
            .build()?;

        let response = match self.client.chat().create(request).await {
            Ok(s) => s,
            Err(e) => return Err(e),
        };

        let mut result = vec![];
        response.choices.iter().for_each(|choice| {
            result.push(choice.message.content.as_ref().unwrap().to_string());
        });

        if let Option::Some(usage) = response.usage {
            debug!(
                "usage: completion_tokens: {}, prompt_tokens: {}, total_tokens: {}",
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
    use tracing::error;

    use super::*;
    use crate::git::Git;

    fn setup_repo() -> Result<Git, Box<dyn Error>> {
        let repo_path = std::env::var("TEST_REPO_PATH")
            .map_err(|_| "TEST_REPO_PATH environment variable not set")?;
        if repo_path.is_empty() {
            return Err("Please specify the repository path".into());
        }

        Git::new(&repo_path)
    }

    #[test]
    fn test_prompt() {
        let repo = setup_repo();
        if repo.is_err() {
            error!("Please specify the repository path");
            return;
        }

        assert!(repo.is_ok());
        let repo = repo.unwrap();

        let diffs = repo.get_diff();
        assert!(diffs.is_ok());

        let logs = repo.get_logs(5);
        assert!(logs.is_ok());

        let diff_content = diffs.unwrap();
        assert!(diff_content.len() > 0);

        let logs_content = logs.unwrap();
        assert!(logs_content.len() > 0);

        let result = OpenAI::prompt(&logs_content, &diff_content).unwrap();
        assert!(!result.is_empty());
    }
}
