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
 * Last Modified: 2025-09-26 15:51:48
 */

use crate::built_info;
use crate::utils::env;
use askama::Template;
use async_openai::config::OPENAI_API_BASE;
use async_openai::error::OpenAIError;
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{ChatCompletionRequestMessage, CreateChatCompletionRequestArgs},
};
use log::trace;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{ClientBuilder, Proxy};
use std::error::Error;
use std::time::Duration;
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
    /// Create a new OpenAI client instance.
    /// This function sets up the OpenAI client with the API key, base URL, and optional proxy settings.
    pub fn new() -> Self {
        // Set up OpenAI client configuration
        let ai_config = OpenAIConfig::new()
            .with_api_key(env::get("OPENAI_API_TOKEN", ""))
            .with_api_base(env::get("OPENAI_API_BASE", OPENAI_API_BASE))
            .with_org_id(built_info::PKG_NAME);

        // Set up HTTP client builder with default headers
        let mut http_client_builder = Self::create_http_client_builder();

        // Set up proxy if specified
        if let Some(proxy_addr) = Self::get_proxy_config() {
            trace!("Using proxy: {proxy_addr}");
            if let Ok(proxy) = Proxy::all(&proxy_addr) {
                http_client_builder = http_client_builder.proxy(proxy);
            }
        }

        // Set up request timeout if specified
        if let Some(timeout) = Self::get_timeout_config() {
            trace!("Setting request timeout to: {timeout}ms");
            http_client_builder = http_client_builder.timeout(Duration::from_millis(timeout));
        }

        // Build the HTTP client
        let http_client = http_client_builder
            .build()
            .expect("Failed to build HTTP client");

        let client = Client::with_config(ai_config).with_http_client(http_client);
        Self { client }
    }

    /// Create HTTP client builder with default headers
    #[inline]
    fn create_http_client_builder() -> ClientBuilder {
        let mut headers = HeaderMap::new();
        headers.insert(
            "HTTP-Referer",
            HeaderValue::from_static(built_info::PKG_HOMEPAGE),
        );
        headers.insert("X-Title", HeaderValue::from_static(built_info::PKG_NAME));
        headers.insert("X-Client-Type", HeaderValue::from_static("CLI"));

        ClientBuilder::new()
            .user_agent(format!(
                "{} ({})",
                built_info::PKG_NAME,
                built_info::PKG_DESCRIPTION
            ))
            .default_headers(headers)
    }

    /// Get proxy configuration from environment
    #[inline]
    fn get_proxy_config() -> Option<String> {
        let proxy_addr = env::get("OPENAI_API_PROXY", "");
        (!proxy_addr.is_empty()).then_some(proxy_addr)
    }

    /// Get timeout configuration from environment
    #[inline]
    fn get_timeout_config() -> Option<u64> {
        let timeout_str = env::get("OPENAI_REQUEST_TIMEOUT", "");
        timeout_str.parse::<u64>().ok()
    }

    /// Check if the OpenAI API and specified model are reachable and available.
    pub async fn check_model(&self, model_name: &str) -> Result<(), Box<dyn Error>> {
        let list = self.client.models().list().await?;

        debug!(
            "Available models: {:?}",
            list.data.iter().map(|m| &m.id).collect::<Vec<_>>()
        );

        if list.data.iter().any(|model| model.id == model_name) {
            debug!("OpenAI API is reachable and model {model_name} is available");
            Ok(())
        } else {
            Err(format!("Model {model_name} not found").into())
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

        trace!("✨ Using model: {}", model_name);

        let response = self.client.chat().create(request).await?;

        let result: Vec<String> = response
            .choices
            .iter()
            .filter_map(|choice| choice.message.content.as_ref().map(ToString::to_string))
            .collect();

        if let Some(usage) = response.usage {
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

        Ok(template.render()?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::git::repository::Repository;
    use tracing::error;

    fn setup_repo() -> Result<Repository, Box<dyn Error>> {
        let repo_path = std::env::var("TEST_REPO_PATH")
            .map_err(|_| "TEST_REPO_PATH environment variable not set")?;
        if repo_path.is_empty() {
            return Err("Please specify the repository path".into());
        }

        Repository::new(&repo_path)
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
        assert!(!diff_content.is_empty());

        let logs_content = logs.unwrap();
        assert!(!logs_content.is_empty());

        let result = OpenAI::prompt(&logs_content, &diff_content).unwrap();
        assert!(!result.is_empty());
    }
}
