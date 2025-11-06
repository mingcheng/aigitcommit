/*!
 * Copyright (c) 2025 Hangzhou Guanwaii Technology Co., Ltd.
 *
 * This source code is licensed under the MIT License,
 * which is located in the LICENSE file in the source tree's root directory.
 *
 * File: utils.rs
 * Author: mingcheng <mingcheng@apache.org>
 * File Created: 2025-10-21 11:34:11
 *
 * Modified By: mingcheng <mingcheng@apache.org>
 * Last Modified: 2025-11-07 11:22:27
 */

use std::env;
use std::io::Write;
use tracing::debug;

use crate::git::message::GitMessage;
use crate::git::repository::Repository;

/// Get environment variable with default value fallback
pub fn get_env(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Parse boolean environment variable
/// Accepts "1", "true", "yes", "on" (case-insensitive) as true
pub fn get_env_bool(key: &str) -> bool {
    env::var(key)
        .map(|v| {
            v == "1"
                || v.eq_ignore_ascii_case("true")
                || v.eq_ignore_ascii_case("yes")
                || v.eq_ignore_ascii_case("on")
        })
        .unwrap_or(false)
}

/// Check if commit should be signed off
/// Returns true if either CLI flag is set or repository/git config/env enable sign-off
pub fn should_signoff(repository: &Repository, cli_signoff: bool) -> bool {
    cli_signoff || repository.should_signoff()
}

/// Output format for commit messages
#[derive(Debug)]
pub enum OutputFormat {
    Stdout,
    Table,
    Json,
}

impl OutputFormat {
    /// Detect output format from CLI flags
    pub fn detect(json: bool, no_table: bool) -> Self {
        if json {
            Self::Json
        } else if no_table {
            Self::Stdout
        } else {
            Self::Table
        }
    }

    /// Write the message in the specified format
    pub fn write(&self, message: &GitMessage) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Self::Stdout => {
                writeln!(std::io::stdout(), "{}", message)?;
            }
            Self::Json => {
                let json = serde_json::to_string_pretty(message)?;
                writeln!(std::io::stdout(), "{}", json)?;
            }
            Self::Table => {
                print_table(&message.title, &message.content);
            }
        }
        Ok(())
    }
}

/// Print the commit message in a table format
pub fn print_table(title: &str, content: &str) {
    let mut binding =
        tabled::builder::Builder::from_iter([["Title", title.trim()], ["Content", content.trim()]])
            .build();
    let table = binding
        .with(tabled::settings::Style::rounded())
        .with(tabled::settings::Width::wrap(120))
        .with(tabled::settings::Alignment::left());

    println!("{}", table);
}

/// Check and print environment variable value
fn check_and_print_env(var_name: &str) {
    match env::var(var_name) {
        Ok(value) => {
            debug!("{} is set to {}", var_name, value);
            println!("{:20}\t{}", var_name, value);
        }
        Err(_) => {
            debug!("{} is not set", var_name);
        }
    }
}

/// Check and print all relevant environment variables
pub fn check_env_variables() {
    [
        "OPENAI_API_BASE",
        "OPENAI_API_TOKEN",
        "OPENAI_MODEL_NAME",
        "OPENAI_API_PROXY",
        "OPENAI_API_TIMEOUT",
        "OPENAI_API_MAX_TOKENS",
        "AIGITCOMMIT_SIGNOFF",
    ]
    .iter()
    .for_each(|v| check_and_print_env(v));
}

/// Convert OpenAI error to user-friendly error message
pub fn format_openai_error(error: async_openai::error::OpenAIError) -> String {
    use async_openai::error::OpenAIError;

    match error {
        OpenAIError::Reqwest(_) | OpenAIError::StreamError(_) => {
            "network request error".to_string()
        }
        OpenAIError::JSONDeserialize(_error, message) => {
            format!("json deserialization error: {message}")
        }
        OpenAIError::InvalidArgument(_) => "invalid argument".to_string(),
        OpenAIError::FileSaveError(_) | OpenAIError::FileReadError(_) => "io error".to_string(),
        OpenAIError::ApiError(e) => format!("api error {e:?}"),
    }
}

/// Save content to a file
pub fn save_to_file(
    path: &str,
    content: &dyn std::fmt::Display,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create(path)?;
    file.write_all(content.to_string().as_bytes())?;
    file.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_table_simple() {
        print_table(
            "Test Title",
            "This is a test content for the commit message.",
        );
    }

    #[test]
    fn test_print_table_with_message() {
        const TITLE: &str = r#"feat: bump version to 1.4.0 and update system template 🚀"#;
        const CONTENT: &str = r#"
- Update version from 1.3.3 to 1.4.0 in Cargo.toml
- Enhance system template with additional instructions
- Simplify and clarify template content for better usability
- Remove redundant information to streamline template
- Ensure template aligns with latest commit message standards

Signed-off-by: mingcheng <mingcheng@apache.org>
        "#;
        print_table(TITLE, CONTENT);
    }

    #[test]
    fn test_get_env() {
        let result = get_env("NONEXISTENT_VAR_XYZ", "default_value");
        assert_eq!(result, "default_value");
    }
}
