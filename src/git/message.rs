/*!
 * Copyright (c) 2026 Ming Lyu, aka mingcheng
 *
 * This source code is licensed under the MIT License,
 * which is located in the LICENSE file in the source tree's root directory.
 *
 * File: message.rs
 * Author: mingcheng <mingcheng@apache.org>
 * File Created: 2025-10-16 15:06:58
 *
 * Modified By: mingcheng <mingcheng@apache.org>
 * Last Modified: 2026-07-30 10:36:45
 */

use crate::built_info::{PKG_NAME, PKG_VERSION};
use crate::git::repository::Repository;
use std::fmt::Write as _;
use std::sync::LazyLock;
use std::{error::Error, fmt::Display};
use tracing::trace;

/// Represents a structured Git commit message
///
/// A commit message consists of:
/// - `title`: The first line (subject line), typically 50-72 characters
/// - `content`: The body of the commit message with detailed description
#[derive(Debug, serde::Serialize)]
pub struct GitMessage {
    pub title: String,
    pub content: String,
}

/// Configuration used when constructing a [`GitMessage`].
///
/// Grouping the construction parameters into a dedicated config struct keeps
/// [`GitMessage::new`] easy to extend with future options (e.g. trailers,
/// scope, breaking-change markers) without breaking call sites.
#[derive(Debug, Clone, Copy, Default)]
pub struct GitMessageConfig<'a> {
    /// The commit title/subject line (will be trimmed).
    pub title: &'a str,
    /// The commit body/description (will be trimmed).
    pub content: &'a str,
    /// Whether to append a `Signed-off-by` trailer.
    pub signoff: bool,
    /// Whether to append tool attribution when signoff is enabled.
    pub attribution: bool,
}

impl<'a> GitMessageConfig<'a> {
    /// Convenience constructor for the most common fields.
    pub fn new(title: &'a str, content: &'a str, signoff: bool) -> Self {
        Self {
            title,
            content,
            signoff,
            attribution: false,
        }
    }
}

impl Display for GitMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Format as: title\n\ncontent
        write!(f, "{}\n\n{}", self.title, self.content)
    }
}

impl GitMessage {
    /// Normalize common transport formatting and reject unusable model output.
    pub fn normalize_response(raw: &str) -> crate::utils::Result<String> {
        static HEADER: LazyLock<regex::Regex> = LazyLock::new(|| {
            regex::Regex::new(
                r"^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([^()\r\n]+\))?!?: \S[^\r\n]*$",
            ).expect("valid Conventional Commit header regex")
        });
        let normalized = raw.replace("\r\n", "\n");
        crate::utils::validate_commit_text(&normalized, true)?;
        let mut text = normalized.trim();
        for prefix in ["```\n", "```text\n", "```plaintext\n"] {
            if let Some(inner) = text
                .strip_prefix(prefix)
                .and_then(|s| s.strip_suffix("\n```"))
            {
                text = inner.trim();
                break;
            }
        }
        let (title, body) = text
            .split_once("\n\n")
            .or_else(|| text.split_once('\n'))
            .ok_or("invalid model response: expected title, blank line, and bullet body")?;
        let body = body.trim();
        if !HEADER.is_match(title) {
            return Err("invalid model response: expected a Conventional Commit title".into());
        }
        let bullets: Vec<&str> = body.lines().collect();
        if !(1..=5).contains(&bullets.len())
            || bullets.iter().any(|line| {
                !line
                    .strip_prefix("- ")
                    .is_some_and(|text| !text.trim().is_empty())
                    || line.contains("```")
                    || line.contains("Signed-off-by:")
                    || line.contains("Co-authored-by:")
                    || line.contains("Generated-by:")
            })
        {
            return Err(
                "invalid model response: expected one to five non-empty bullets without trailers"
                    .into(),
            );
        }
        Ok(format!("{title}\n\n{body}"))
    }

    /// Create a new Git commit message from a [`GitMessageConfig`].
    ///
    /// # Arguments
    /// * `repository` - The Git repository (used to get author info for signoff)
    /// * `config` - Construction parameters; see [`GitMessageConfig`]
    ///
    /// # Returns
    /// * `Ok(GitMessage)` - A valid commit message
    /// * `Err` - If text is empty, contains unsafe characters, or identity is invalid
    ///
    pub fn new(repository: &Repository, config: GitMessageConfig) -> Result<Self, Box<dyn Error>> {
        crate::utils::validate_commit_text(config.title, false)?;
        crate::utils::validate_commit_text(config.content, true)?;
        // Trim inputs first to check actual content
        let title_trimmed = config.title.trim();
        let content_trimmed = config.content.trim();

        // Validate both title and content are non-empty
        if title_trimmed.is_empty() {
            return Err("commit title cannot be empty".into());
        }
        if content_trimmed.is_empty() {
            return Err("commit content cannot be empty".into());
        }

        // Start with the trimmed content; we'll append signoff if needed
        let mut final_content = content_trimmed.to_string();

        // Append signoff line if requested
        if config.signoff {
            trace!("adding Signed-off-by line to commit message");
            let author = repository.get_author()?;
            // Writing into the existing String avoids the intermediate alloc
            // that `format!` + `push_str` would create.
            write!(
                final_content,
                "\n\nSigned-off-by: {} <{}>",
                author.name, author.email
            )?;
            if config.attribution {
                write!(final_content, "\nGenerated-by: {PKG_NAME} v{PKG_VERSION}")?;
            }
        }

        trace!("created commit message with title: {}", title_trimmed);
        trace!("content length: {} characters", final_content.len());

        Ok(Self {
            title: title_trimmed.to_string(),
            content: final_content,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn normalizes_fenced_crlf_response() {
        assert_eq!(
            GitMessage::normalize_response(
                "```text\r\nfix(core): handle empty input\r\n\r\n- reject empty values\r\n```"
            )
            .unwrap(),
            "fix(core): handle empty input\n\n- reject empty values",
        );
    }

    #[test]
    fn rejects_terminal_controls_and_display_spoofing_before_trimming() {
        let forbidden = (0u8..=0x9f)
            .map(char::from)
            .filter(|ch| ch.is_control() && *ch != '\n')
            .chain([
                '\u{00ad}', '\u{061c}', '\u{200b}', '\u{200e}', '\u{200f}', '\u{2028}', '\u{2029}',
                '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}', '\u{202e}', '\u{2060}', '\u{2066}',
                '\u{2067}', '\u{2068}', '\u{2069}', '\u{feff}',
            ]);
        for ch in forbidden {
            for raw in [
                format!("fix: handle {ch}input\n\n- reject invalid input"),
                format!("fix: handle input\n\n- reject {ch}invalid input"),
                format!("fix: handle input\n\n- reject invalid input{ch}"),
            ] {
                let error = GitMessage::normalize_response(&raw)
                    .unwrap_err()
                    .to_string();
                assert!(error.contains(&format!("U+{:04X}", u32::from(ch))));
                assert!(!error.contains(ch));
            }
        }
    }

    #[test]
    fn preserves_multilingual_prose_and_joiners() {
        for raw in [
            "fix: \u{4fee}\u{590d}\u{8f93}\u{5165}\n\n- \u{68c0}\u{67e5}\u{8f93}\u{5165}",
            "fix: \u{645}\u{6cc}\u{200c}\u{634}\u{648}\u{62f}\n\n- preserve \u{915}\u{94d}\u{200d}\u{937}",
            "fix(api)!: handle input\n\n- preserve \u{1f469}\u{200d}\u{1f4bb}",
        ] {
            assert_eq!(GitMessage::normalize_response(raw).unwrap(), raw);
        }
    }

    #[test]
    fn normalizes_missing_blank_separator_without_accepting_prose() {
        assert_eq!(
            GitMessage::normalize_response("fix: accept zero\n- allow zero counts").unwrap(),
            "fix: accept zero\n\n- allow zero counts",
        );
        assert!(
            GitMessage::normalize_response("fix: accept zero\nExplanation:\n- allow zero").is_err()
        );
    }

    #[test]
    fn rejects_malformed_generated_messages() {
        for raw in [
            "",
            "No changes detected.",
            "fix: x",
            "fix: x\n\n",
            "Here is a message:\n\n- something",
            "fix: x\n\nbody",
            "fix: x\n\n- ",
            "fix: x\n\n- change\nSigned-off-by: someone",
            "fix: x\n\n- Generated-by: invented tool",
            "fix: x\n\n- a\n- b\n- c\n- d\n- e\n- f",
        ] {
            assert!(GitMessage::normalize_response(raw).is_err(), "{raw:?}");
        }
        assert!(
            GitMessage::normalize_response(
                "feat(api)!: change response\n\n- update response fields"
            )
            .is_ok()
        );
    }

    fn setup() -> Option<Repository> {
        let path = env::var("TEST_REPO_PATH").unwrap_or_else(|_| ".".to_string());
        Repository::new(&path).ok()
    }

    #[test]
    fn rejects_empty_title() {
        let Some(repo) = setup() else { return };
        let err = GitMessage::new(&repo, GitMessageConfig::new("   ", "body", false)).unwrap_err();
        assert!(err.to_string().contains("title"));
    }

    #[test]
    fn rejects_empty_content() {
        let Some(repo) = setup() else { return };
        let err = GitMessage::new(&repo, GitMessageConfig::new("title", "   ", false)).unwrap_err();
        assert!(err.to_string().contains("content"));
    }

    #[test]
    fn trims_inputs_and_formats_display() {
        let Some(repo) = setup() else { return };
        let msg = GitMessage::new(
            &repo,
            GitMessageConfig::new("  feat: x  ", "  body line  ", false),
        )
        .unwrap();
        assert_eq!(msg.title, "feat: x");
        assert_eq!(msg.content, "body line");
        assert_eq!(format!("{msg}"), "feat: x\n\nbody line");
    }

    #[test]
    fn appends_signoff_line_when_requested() {
        let Some(repo) = setup() else { return };
        let msg = GitMessage::new(&repo, GitMessageConfig::new("feat: x", "body", true)).unwrap();
        assert!(
            msg.content.contains("Signed-off-by:"),
            "signoff line missing: {}",
            msg.content
        );
        // Signoff is separated from body by a blank line.
        assert!(msg.content.contains("\n\nSigned-off-by:"));
    }
}
