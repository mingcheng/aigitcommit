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

pub const CMD: &str = "aigitcommit";
pub const CMD_ABOUT: &str = "A simple tool to help you write better Git commit messages using AI.";
pub const CMD_ABOUT_URL: &str = "https://github.com/mingcheng/aigitcommit";

#[derive(Debug, Parser)]
#[command(name = CMD)]
#[command(about = CMD_ABOUT, version)]
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
    pub check: bool,

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
    pub copy: bool,

    #[arg(
        long,
        short,
        default_value = "",
        help = "Save the commit message to a file",
        required = false
    )]
    pub save: String,
}
