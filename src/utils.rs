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
 * Last Modified: 2025-10-21 17:52:45
 */

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
}
