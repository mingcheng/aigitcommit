/*
 * Copyright (c) 2025 Hangzhou Guanwaii Technology Co,.Ltd.
 *
 * This source code is licensed under the MIT License,
 * which is located in the LICENSE file in the source tree's root directory.
 *
 * File: lib.rs
 * Author: mingcheng (mingcheng@apache.org)
 * File Created: 2025-03-01 21:56:02
 *
 * Modified By: mingcheng (mingcheng@apache.org)
 * Last Modified: 2025-03-03 19:36:07
 */

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub mod cli;
pub mod git;
pub mod openai;
pub mod utils;
