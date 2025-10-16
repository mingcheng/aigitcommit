/*!
 * Copyright (c) 2025 Hangzhou Guanwaii Technology Co,.Ltd.
 *
 * This source code is licensed under the MIT License,
 * which is located in the LICENSE file in the source tree's root directory.
 *
 * File: build.rs
 * Author: mingcheng <mingcheng@apache.org>
 * File Created: 2025-10-16 13:46:57
 *
 * Modified By: mingcheng <mingcheng@apache.org>
 * Last Modified: 2025-10-16 13:47:06
 */

fn main() {
    built::write_built_file().expect("Failed to write built.rs");
}
