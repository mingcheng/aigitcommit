/*!
 * Copyright (c) 2025 Hangzhou Guanwaii Technology Co., Ltd.
 *
 * This source code is licensed under the MIT License,
 * which is located in the LICENSE file in the source tree's root directory.
 *
 * File: message.rs
 * Author: mingcheng <mingcheng@apache.org>
 * File Created: 2025-10-16 15:06:58
 *
 * Modified By: mingcheng <mingcheng@apache.org>
 * Last Modified: 2025-10-17 18:22:55
 */

use crate::git::repository::Repository;
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

impl Display for GitMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Format as: title\n\ncontent
        write!(f, "{}\n\n{}", self.title, self.content)
    }
}

impl GitMessage {
    /// Create a new Git commit message
    ///
    /// # Arguments
    /// * `repository` - The Git repository (used to get author info for signoff)
    /// * `title` - The commit title/subject line (will be trimmed)
    /// * `content` - The commit body/description (will be trimmed)
    /// * `signoff` - Whether to append a "Signed-off-by" line
    ///
    /// # Returns
    /// * `Ok(GitMessage)` - A valid commit message
    /// * `Err` - If title or content is empty after trimming
    ///
    pub fn new(
        repository: &Repository,
        title: &str,
        content: &str,
        signoff: bool,
    ) -> Result<Self, Box<dyn Error>> {
        // Trim inputs first to check actual content
        let title_trimmed = title.trim();
        let content_trimmed = content.trim();

        // Validate both title and content are non-empty
        if title_trimmed.is_empty() {
            return Err("commit title cannot be empty".into());
        }
        if content_trimmed.is_empty() {
            return Err("commit content cannot be empty".into());
        }

        let mut final_content = content_trimmed.to_string();

        // Append signoff line if requested
        if signoff {
            trace!("adding Signed-off-by line to commit message");
            let author = repository.get_author()?;

            // Ensure proper spacing before signoff
            final_content.push_str(&format!(
                "\n\nSigned-off-by: {} <{}>",
                author.name, author.email
            ));
        }

        trace!("created commit message with title: {}", title_trimmed);
        trace!("content length: {} characters", final_content.len());

        Ok(Self {
            title: title_trimmed.to_string(),
            content: final_content,
        })
    }

    /// Check if the commit message is empty
    ///
    /// Returns true only if both title and content are empty strings
    pub fn is_empty(&self) -> bool {
        self.title.is_empty() && self.content.is_empty()
    }

    /// Get the total character count of the commit message
    pub fn char_count(&self) -> usize {
        self.title.len() + 2 + self.content.len() // +2 for "\n\n"
    }

    /// Get the number of lines in the commit message
    pub fn line_count(&self) -> usize {
        1 + self.content.lines().count() // +1 for title, +blank line is implicit
    }
}
