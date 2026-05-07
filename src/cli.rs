/*
 * Copyright (c) 2025-2026 mingcheng <mingcheng@apache.org>
 *
 * This source code is licensed under the MIT License,
 * which is located in the LICENSE file in the source tree's root directory.
 *
 * File: cli.rs
 * Author: mingcheng (mingcheng@apache.org)
 * File Created: 2025-03-03 19:31:27
 *
 * Modified By: mingcheng (mingcheng@apache.org)
 * Last Modified: 2025-03-05 00:25:24
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
    version = built_info::PKG_VERSION,
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

    /// Prompt to commit after generating the message.
    #[arg(long)]
    pub commit: bool,

    /// Append a `Signed-off-by` trailer to the commit message.
    #[arg(long)]
    pub signoff: bool,

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

    /// Print the values of the OpenAI-related environment variables and exit.
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
}
