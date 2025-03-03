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
 * Last Modified: 2025-03-03 19:47:04
 */

use clap::Parser;

pub const CMD: &str = "aigitcommit";
pub const CMD_ABOUT: &str = "A simple tool to help you write better Git commit messages using AI.";

#[derive(Debug, Parser)]
#[command(name = CMD)]
#[command(about = CMD_ABOUT, version)]
pub struct Cli {
    /// Specify the file path to repository directory.
    /// If not specified, the current directory will be used
    #[arg(default_value = ".", required = false)]
    pub repo_path: String,

    /// Verbose mode
    #[arg(long, short, default_value_t = false, required = false)]
    pub verbose: bool,
}
