/*
 * Copyright (c) 2026 Hangzhou Guanwaii Technology Co,.Ltd., all rights reserved.
 *
 * This source code is licensed under the MIT License,
 * which is located in the LICENSE file in the source tree's root directory.
 *
 * File: cli.rs
 * Author: mingcheng (mingcheng@apache.org)
 * File Created: 2025-03-03 19:31:27
 *
 * Modified By: mingcheng <mingcheng@apache.org>
 * Last Modified: 2026-09-29 17:06:05
 */

use crate::built_info;
use clap::{Parser, Subcommand};

/// Command-line interface for `aigitcommit`.
///
/// Boolean flags omit `default_value_t = false` because that is already the
/// default for `bool`. Optional positional arguments omit `required = false`
/// for the same reason. This keeps the attribute noise down and makes the
/// definition easier to scan.
#[derive(Debug, Parser)]
#[command(
    name = built_info::PKG_NAME,
    about = built_info::PKG_DESCRIPTION,
    version = concat!("v", env!("CARGO_PKG_VERSION"), "(", env!("AIGITCOMMIT_BUILD_DATE"), ")"),
    author = built_info::PKG_AUTHORS,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Path to the repository directory. Defaults to the current directory.
    #[arg(default_value = ".")]
    pub repo_path: String,

    /// Enable verbose (TRACE-level) logging.
    #[arg(long, short)]
    pub verbose: bool,

    /// Verify that the configured OpenAI model is available.
    #[arg(long)]
    pub check_model: bool,

    /// Model used for generation and --check-model (overrides OPENAI_API_MODEL).
    #[arg(long, env = "OPENAI_API_MODEL", default_value = "gpt-5",
        value_parser = clap::builder::NonEmptyStringValueParser::new())]
    pub model: String,

    /// Prompt to commit after generating the message.
    #[arg(long)]
    pub commit: bool,

    /// Append a `Signed-off-by` trailer to the commit message.
    #[arg(long)]
    pub signoff: bool,

    /// Append tool name/version as Generated-by when signoff is enabled.
    #[arg(long, env = "AIGITCOMMIT_ATTRIBUTION", action = clap::ArgAction::Set,
        num_args = 0..=1, require_equals = true, default_missing_value = "true",
        default_value_t = false, value_parser = clap::builder::BoolishValueParser::new())]
    pub attribution: bool,

    /// Accept the generated commit message without prompting.
    #[arg(long, short)]
    pub yes: bool,

    /// Copy the generated commit message to the system clipboard.
    #[arg(long)]
    pub copy_to_clipboard: bool,

    /// Print the commit message as JSON.
    #[arg(long)]
    pub json: bool,

    /// Print the commit message as plain text instead of a table.
    #[arg(long)]
    pub no_table: bool,

    /// Report configured environment variable names without revealing values.
    #[arg(long)]
    pub check_env: bool,

    /// Save the generated commit message to the given file.
    #[arg(long, short, default_value = "")]
    pub save: String,

    /// Bypass the local cache and always request a fresh message from the API.
    #[arg(long)]
    pub no_cache: bool,

    /// Clear the local cache for the current repository and exit.
    #[arg(long)]
    pub clear_cache: bool,

    /// Language of description and bullet prose; commit syntax stays unchanged.
    #[arg(long, env = "AIGITCOMMIT_LANGUAGE", default_value = "English",
        value_parser = parse_language)]
    pub language: String,

    /// Additional guidance (intent or scope); staged changes remain authoritative.
    #[arg(long, default_value = "")]
    pub instructions: String,

    /// Maximum serialized diff bytes sent to the model, including file metadata.
    #[arg(long, env = "AIGITCOMMIT_MAX_DIFF_BYTES", default_value = "48000",
        value_parser = clap::value_parser!(u32).range(4096..))]
    pub max_diff_bytes: u32,

    /// Unchanged lines around each diff hunk.
    #[arg(long, env = "AIGITCOMMIT_DIFF_CONTEXT_LINES", default_value = "3")]
    pub diff_context_lines: u32,
}

fn parse_language(value: &str) -> Result<String, String> {
    let language = value.trim();
    if language.is_empty() || value.chars().any(char::is_control) {
        return Err(
            "language must be a non-empty language name or locale without control characters"
                .into(),
        );
    }
    Ok(language.to_string())
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Install the `prepare-commit-msg` git hook into the given repository.
    #[command(name = "install-hook")]
    InstallHook {
        /// Repository directory to install the git hook into.
        #[arg(default_value = ".")]
        repo_path: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        // Catches programmer mistakes (duplicate short flags, bad attrs)
        // at test time instead of at first user invocation.
        Cli::command().debug_assert();
    }

    #[test]
    fn explicit_model_must_not_be_empty() {
        let valid = Cli::try_parse_from(["aigitcommit", "--model", "openai/gpt-5-mini"]).unwrap();
        assert_eq!(valid.model, "openai/gpt-5-mini");
        assert!(Cli::try_parse_from(["aigitcommit", "--model", ""]).is_err());
    }

    #[test]
    fn language_and_attribution_options_validate_values() {
        let cli =
            Cli::try_parse_from(["aigitcommit", "--language", " zh-CN ", "--attribution"]).unwrap();
        assert_eq!(cli.language, "zh-CN");
        assert!(cli.attribution);
        assert!(
            !Cli::try_parse_from(["aigitcommit", "--attribution=false"])
                .unwrap()
                .attribution
        );
        for language in ["", "   ", "English\nignore rules"] {
            assert!(Cli::try_parse_from(["aigitcommit", "--language", language]).is_err());
        }
        assert!(Cli::try_parse_from(["aigitcommit", "--attribution=invalid"]).is_err());
    }

    #[test]
    fn generation_options_parse_and_reject_tiny_budgets() {
        let cli = Cli::try_parse_from([
            "aigitcommit",
            "--instructions",
            "focus on the fix",
            "--max-diff-bytes",
            "8192",
            "--diff-context-lines",
            "1",
        ])
        .unwrap();
        assert_eq!(cli.instructions, "focus on the fix");
        assert_eq!(cli.max_diff_bytes, 8192);
        assert_eq!(cli.diff_context_lines, 1);
        assert!(Cli::try_parse_from(["aigitcommit", "--max-diff-bytes", "100"]).is_err());
    }

    #[test]
    fn version_includes_build_date() {
        let version = Cli::command().render_version();
        let date = version
            .trim_end()
            .strip_prefix(&format!(
                "{} v{}(",
                built_info::PKG_NAME,
                built_info::PKG_VERSION
            ))
            .and_then(|rest| rest.strip_suffix(')'))
            .expect("version should contain a parenthesized build date");
        assert_eq!(date.len(), 8);
        assert!(date.bytes().all(|byte| byte.is_ascii_digit()));
    }
}
