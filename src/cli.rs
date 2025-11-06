/*
 * Copyright (c) 2025 Hangzhou Guanwaii Technology Co,.Ltd.
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

use clap::Parser;
use crate::built_info;

#[derive(Debug, Parser)]
#[command(name = built_info::PKG_NAME, about = built_info::PKG_DESCRIPTION, version = built_info::PKG_VERSION, author = built_info::PKG_AUTHORS)]
pub struct Cli {
    #[arg(
        default_value = ".",
        help = r#"Specify the file path to repository directory.
If not specified, the current directory will be used"#,
        required = false
    )]
    pub repo_path: String,

    #[arg(
        long,
        short,
        help = "Verbose mode",
        default_value_t = false,
        required = false
    )]
    pub verbose: bool,

    #[arg(
        long,
        help = "Check the openai api key and model name whether is available",
        default_value_t = false,
        required = false
    )]
    pub check_model: bool,

    #[arg(
        long,
        help = "Prompt the commit after generating the message",
        default_value_t = false,
        required = false
    )]
    pub commit: bool,

    #[arg(
        long,
        help = "Mark whether the commit is a signoff commit",
        default_value_t = false,
        required = false
    )]
    pub signoff: bool,

    #[arg(
        long,
        short,
        help = "Accept the commit message without prompting",
        default_value_t = false,
        required = false
    )]
    pub yes: bool,

    #[arg(
        long,
        help = "Copy the commit message to clipboard",
        default_value_t = false,
        required = false
    )]
    pub copy_to_clipboard: bool,

    #[arg(
        long,
        help = "Print the commit message in JSON format",
        default_value_t = false,
        required = false
    )]
    pub json: bool,

    #[arg(
        long,
        help = "Print the commit message in a table format",
        default_value_t = false,
        required = false
    )]
    pub no_table: bool,

    #[arg(
        long,
        help = "Check current environment variables for OpenAI API key and model name",
        default_value_t = false,
        required = false
    )]
    pub check_env: bool,

    #[arg(
        long,
        short,
        default_value = "",
        help = "Save the commit message to a file",
        required = false
    )]
    pub save: String,
}

#[cfg(test)]
mod tests {}
