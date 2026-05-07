/*!
 * Copyright (c) 2025-2026 mingcheng <mingcheng@apache.org>
 *
 * This source code is licensed under the MIT License,
 * which is located in the LICENSE file in the source tree's root directory.
 *
 * File: utils.rs
 * Author: mingcheng <mingcheng@apache.org>
 * File Created: 2025-10-21 11:34:11
 *
 * Modified By: mingcheng <mingcheng@apache.org>
 * Last Modified: 2026-05-07 11:29:44
 */

use crate::git::message::GitMessage;
use crate::git::repository::Repository;
use std::fs;
use std::io::Write;
use tracing::trace;

/// Generic result type for utility functions
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Get environment variable with default value fallback
pub mod env {
    use std::env;

    use tracing::{debug, warn};

    /// Get environment variable with default value fallback
    pub fn get(key: &str, default: &str) -> String {
        env::var(key).unwrap_or_else(|_| default.to_string())
    }

    /// Parse boolean environment variable
    /// Accepts "1", "true", "yes", "on" (case-insensitive) as true
    pub fn get_bool(key: &str) -> bool {
        env::var(key)
            .map(|v| {
                v == "1"
                    || v.eq_ignore_ascii_case("true")
                    || v.eq_ignore_ascii_case("yes")
                    || v.eq_ignore_ascii_case("on")
            })
            .unwrap_or(false)
    }

    /// Check and print environment variable value
    pub fn exists(var_name: &str) -> bool {
        match env::var(var_name) {
            Ok(value) => {
                debug!("{} is set to {}", var_name, value);
                // println!("{:20}\t{}", var_name, value);
                true
            }
            Err(_) => {
                warn!("{} is not set", var_name);
                false
            }
        }
    }
}

/// Check if commit should be signed off
/// Returns true if either CLI flag is set or repository/git config/env enable sign-off
pub fn should_signoff(repository: &Repository, cli_signoff: bool) -> bool {
    cli_signoff || repository.should_signoff()
}

/// Output format for commit messages
#[derive(Debug, PartialEq, Eq)]
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
    pub fn write(&self, message: &GitMessage) -> Result<()> {
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
    .for_each(|v| {
        env::exists(v);
    });
}

/// Save content to a file
pub fn save_to_file(path: &str, content: &dyn std::fmt::Display) -> Result<()> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create(path)?;
    file.write_all(content.to_string().as_bytes())?;
    file.flush()?;
    Ok(())
}

/// Install the prepare-commit-msg git hook into the target repository.
pub fn install_hook(path: &str, name: &str, content: &str) -> Result<()> {
    let repo_dir =
        fs::canonicalize(path).map_err(|e| format!("resolve repository path failed: {e}"))?;
    let git_dir = repo_dir.join(".git");
    if !git_dir.is_dir() {
        return Err("not a git repository (missing .git directory)".into());
    }

    let hooks_dir = git_dir.join("hooks");
    fs::create_dir_all(&hooks_dir).map_err(|e| format!("create hooks dir failed: {e}"))?;

    let hook_path = hooks_dir.join(name);
    fs::write(&hook_path, content).map_err(|e| format!("write hook file failed: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path)
            .map_err(|e| format!("get hook metadata failed: {e}"))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)
            .map_err(|e| format!("set executable permission failed: {e}"))?;
    }

    trace!("hook installed at {:?}", hook_path);
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
        let result = env::get("NONEXISTENT_VAR_XYZ", "default_value");
        assert_eq!(result, "default_value");
    }

    #[test]
    fn test_get_bool_truthy_and_falsy() {
        // SAFETY: tests run in the same process; use uniquely-named keys.
        unsafe {
            std::env::set_var("AIGC_TEST_BOOL_T1", "1");
            std::env::set_var("AIGC_TEST_BOOL_T2", "TRUE");
            std::env::set_var("AIGC_TEST_BOOL_T3", "Yes");
            std::env::set_var("AIGC_TEST_BOOL_T4", "on");
            std::env::set_var("AIGC_TEST_BOOL_F1", "0");
            std::env::set_var("AIGC_TEST_BOOL_F2", "no");
        }
        assert!(env::get_bool("AIGC_TEST_BOOL_T1"));
        assert!(env::get_bool("AIGC_TEST_BOOL_T2"));
        assert!(env::get_bool("AIGC_TEST_BOOL_T3"));
        assert!(env::get_bool("AIGC_TEST_BOOL_T4"));
        assert!(!env::get_bool("AIGC_TEST_BOOL_F1"));
        assert!(!env::get_bool("AIGC_TEST_BOOL_F2"));
        assert!(!env::get_bool("AIGC_TEST_BOOL_MISSING"));
    }

    #[test]
    fn test_output_format_detect() {
        assert_eq!(OutputFormat::detect(true, false), OutputFormat::Json);
        assert_eq!(OutputFormat::detect(true, true), OutputFormat::Json);
        assert_eq!(OutputFormat::detect(false, true), OutputFormat::Stdout);
        assert_eq!(OutputFormat::detect(false, false), OutputFormat::Table);
    }

    #[test]
    fn test_save_to_file_roundtrip() {
        let path = std::env::temp_dir()
            .join(format!("aigitcommit-save-{}.txt", std::process::id()));
        let path_str = path.to_string_lossy().into_owned();
        save_to_file(&path_str, &"hello world").unwrap();
        let read = std::fs::read_to_string(&path).unwrap();
        assert_eq!(read, "hello world");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_install_hook() {
        let result = install_hook(".", "test-hook", "#!/bin/sh\necho 'Test Hook'");
        assert!(result.is_ok());
    }

    #[test]
    fn test_install_hook_rejects_non_git_dir() {
        let tmp = std::env::temp_dir()
            .join(format!("aigitcommit-not-a-repo-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let result = install_hook(tmp.to_str().unwrap(), "x", "#!/bin/sh\n");
        assert!(result.is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
