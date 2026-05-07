/*!
 * Copyright (c) 2025-2026 mingcheng <mingcheng@apache.org>
 *
 * This source code is licensed under the MIT License,
 * which is located in the LICENSE file in the source tree's root directory.
 *
 * File: repository.rs
 * Author: mingcheng <mingcheng@apache.org>
 * File Created: 2025-10-16 15:07:05
 *
 * Modified By: mingcheng <mingcheng@apache.org>
 * Last Modified: 2026-05-07 11:30:38
 */

use git2::{Oid, Repository as _Repo, RepositoryOpenFlags, Signature};
use regex::Regex;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::sync::LazyLock;
use tracing::{trace, warn};

use crate::git::message::GitMessage;
use crate::utils::env;

/// Files commonly auto-generated or noisy that should be excluded from the
/// diff sent to the model.
const EXCLUDED_FILES: &[&str] = &[
    "go.mod",
    "go.sum",
    "Cargo.lock",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
];

/// Compiled once: validates a minimally well-formed `local@domain.tld` email.
static EMAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$").expect("valid email regex"));

/// Author information from git configuration
pub struct Author {
    pub name: String,
    pub email: String,
}

/// Git repository wrapper providing high-level operations
pub struct Repository {
    repository: _Repo,
}

impl Display for Repository {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Git repository at {}", self.repository.path().display())
    }
}

impl Repository {
    /// Create a new Git repository wrapper
    ///
    /// # Arguments
    /// * `path` - Path to the git repository (can be a subdirectory within the repo)
    ///
    /// # Returns
    /// * `Ok(Git)` - Successfully opened repository
    /// * `Err` - Repository not found or has no working directory (bare repo)
    pub fn new(path: &str) -> Result<Repository, Box<dyn Error>> {
        trace!("opening repository at {path}");
        // Allow upward discovery from `path` so callers can pass any
        // subdirectory inside a working tree. `ceiling_dirs` is empty so
        // discovery walks up to the filesystem root or first `.git`.
        let no_ceilings: [&str; 0] = [];
        let repository =
            _Repo::open_ext(path, RepositoryOpenFlags::empty(), no_ceilings.iter().copied())?;

        trace!("repository opened successfully");
        if let Some(work_dir) = repository.workdir() {
            trace!("the repository workdir is: {work_dir:?}");
        } else {
            return Err(
                "the repository has no workdir (bare repositories are not supported)".into(),
            );
        }

        Ok(Repository { repository })
    }

    /// Get the path to the underlying `.git` directory.
    ///
    /// Useful for storing repository-scoped state (e.g. local caches).
    pub fn git_dir(&self) -> &Path {
        self.repository.path()
    }

    /// Commit the staged changes in the repository
    ///
    /// # Arguments
    /// * `message` - The commit message to use
    ///
    /// # Returns
    /// * `Ok(())` - Commit created successfully
    /// * `Err` - Failed to create commit (no staged changes, invalid author info, etc.)
    pub fn commit(&self, message: &GitMessage) -> Result<Oid, Box<dyn Error>> {
        let message = message.to_string();
        let mut index = self.repository.index()?;

        // Write the current index (staged changes) to a tree object
        let oid = index.write_tree()?;
        let tree = self.repository.find_tree(oid)?;

        // Get the parent commit(s) - handle both initial commit and subsequent commits
        let parents = match self.repository.head() {
            Ok(head_ref) => {
                // Repository has commits, use HEAD as parent
                let head_commit = head_ref.peel_to_commit()?;
                vec![head_commit]
            }
            Err(e) if e.code() == git2::ErrorCode::UnbornBranch => {
                // Initial commit - no parents
                trace!("creating initial commit (no parent commits)");
                vec![]
            }
            Err(e) => return Err(Box::new(e)),
        };

        // Get author information from git config
        let author = self.get_author()?;

        // Create a signature with current timestamp
        let signature = Signature::now(&author.name, &author.email)?;

        // Create the commit with parent references
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
        let result = self.repository.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &message,
            &tree,
            &parent_refs,
        )?;

        Ok(result)
    }

    /// Get the author email and name from the repository configuration
    ///
    /// Attempts to read user.name and user.email from git config.
    /// Falls back to environment variables or defaults if not configured.
    ///
    /// # Returns
    /// * `Ok(Author)` - Author information retrieved successfully
    /// * `Err` - Failed to read configuration
    pub fn get_author(&self) -> Result<Author, Box<dyn Error>> {
        let config = self.repository.config()?;

        // Default email if none found
        const UNKNOWN_EMAIL: &str = "unknown@users.noreply.github.com";
        const UNKNOWN_AUTHOR: &str = "Unknown Author";

        // Try to get user.email from config, fall back to environment or default
        let email = config
            .get_string("user.email")
            .or_else(|_| {
                warn!("user.email not configured in git config");
                std::env::var("GIT_AUTHOR_EMAIL")
            })
            .unwrap_or_else(|_| {
                warn!("using default email: {}", UNKNOWN_EMAIL);
                env::get("GIT_FALLBACK_EMAIL", UNKNOWN_EMAIL)
            });

        // Validate email format using regex
        let email = if EMAIL_RE.is_match(&email) {
            email
        } else {
            warn!("invalid email format: {}, using default", email);
            env::get("GIT_FALLBACK_EMAIL", UNKNOWN_EMAIL)
        };

        // Try to get user.name from config, fall back to environment or default
        let name = config
            .get_string("user.name")
            .or_else(|_| {
                warn!("user.name not configured in git config");
                std::env::var("GIT_AUTHOR_NAME")
            })
            .unwrap_or_else(|_| {
                warn!("using default name: Unknown User");
                "Unknown User".to_string()
            });

        // Detect if name is empty or whitespace
        let name = if name.trim().is_empty() {
            warn!("author name is empty, using default: Unknown Author");
            env::get("GIT_FALLBACK_NAME", UNKNOWN_AUTHOR)
        } else {
            name
        };

        Ok(Author { name, email })
    }

    /// Get the diff of the staged changes (index vs HEAD)
    ///
    /// Returns the patch format diff, excluding certain lock files.
    /// Filters out: go.mod, go.sum, Cargo.lock, package-lock.json, yarn.lock, pnpm-lock.yaml
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - Lines of the diff in patch format
    /// * `Err` - Failed to generate diff
    pub fn get_diff(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let index = self.repository.index()?;

        // Get the HEAD tree, or None for initial commit
        let head_tree = match self.repository.head() {
            Ok(head_ref) => Some(head_ref.peel_to_commit()?.tree()?),
            Err(e) if e.code() == git2::ErrorCode::UnbornBranch => {
                trace!("generating diff for initial commit");
                None
            }
            Err(e) => return Err(Box::new(e)),
        };

        // Configure diff options
        let mut diffopts = git2::DiffOptions::new();
        diffopts
            .show_binary(false)
            .force_binary(false)
            .ignore_submodules(true)
            .minimal(true)
            .context_lines(3);

        // Generate diff between HEAD and index (staged changes)
        let diff = self.repository.diff_tree_to_index(
            head_tree.as_ref(),
            Some(&index),
            Some(&mut diffopts),
        )?;

        let excluded_files = Self::get_excluded_files();
        let mut result = Vec::new();

        diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
            // Check if the file should be excluded
            let should_exclude = delta
                .new_file()
                .path()
                .and_then(|p| p.file_name())
                .map(|f| excluded_files.contains(&f.to_string_lossy().as_ref()))
                .unwrap_or(false);

            if should_exclude {
                if let Some(filename) = delta.new_file().path().and_then(|p| p.file_name()) {
                    warn!("skipping excluded file: {}", filename.to_string_lossy());
                }
                return true;
            }

            // Add non-empty lines to result
            let content = String::from_utf8_lossy(line.content());
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                result.push(trimmed.to_string());
            }
            true
        })?;

        Ok(result)
    }

    /// Check if commit should be signed off
    /// Returns true when git config `aigitcommit.signoff` is enabled or the
    /// `AIGITCOMMIT_SIGNOFF` environment variable evaluates to true.
    ///
    /// The env variable is consulted whenever the config key is missing or
    /// not set to true, so users can opt in globally without touching git
    /// config in every repository.
    pub fn should_signoff(&self) -> bool {
        // Define the config key for signoff
        const SIGNOFF_KEY: &str = "aigitcommit.signoff";

        let from_config = self
            .repository
            .config()
            .ok()
            .and_then(|c| c.get_bool(SIGNOFF_KEY).ok())
            .unwrap_or(false);
        trace!("✍️ git config signoff: {}", from_config);

        from_config || env::get_bool("AIGITCOMMIT_SIGNOFF")
    }

    /// Get the list of filenames to exclude from diffs.
    #[inline]
    fn get_excluded_files() -> &'static [&'static str] {
        EXCLUDED_FILES
    }

    /// Get the latest `size` commit messages from the repository
    ///
    /// Retrieves commit messages in reverse chronological order (newest first).
    ///
    /// # Arguments
    /// * `size` - Maximum number of commit messages to retrieve
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - List of commit messages (may be fewer than `size` if repo has fewer commits)
    /// * `Err` - Failed to walk commit history
    pub fn get_logs(&self, size: usize) -> Result<Vec<String>, Box<dyn Error>> {
        let mut revwalk = self.repository.revwalk()?;

        // Start walking from HEAD
        revwalk.push_head()?;

        // Sort by time (newest first) - this is the default but made explicit
        revwalk.set_sorting(git2::Sort::TIME)?;

        // Collect up to `size` commit messages
        let commits: Vec<String> = revwalk
            .take(size)
            .filter_map(|oid_result| {
                oid_result
                    .ok()
                    .and_then(|oid| self.repository.find_commit(oid).ok())
                    .and_then(|commit| {
                        commit
                            .message()
                            .map(str::trim)
                            .filter(|msg| !msg.is_empty())
                            .map(String::from)
                    })
            })
            .collect();

        trace!("retrieved {} commit messages", commits.len());
        Ok(commits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::error;

    fn setup() -> Result<Repository, Box<dyn Error>> {
        let repo_path = std::env::var("TEST_REPO_PATH").unwrap_or(".".to_string());
        Repository::new(&repo_path)
    }

    #[test]
    fn test_new() {
        if setup().is_err() {
            error!("please specify the repository path");
            return;
        }

        assert!(setup().is_ok());
    }

    #[test]
    fn test_get_author() {
        let repo = setup().unwrap();
        let author = repo.get_author().unwrap();
        assert!(!author.name.is_empty());
        assert!(!author.email.is_empty());
    }

    #[test]
    fn test_logs() {
        let repo = setup();
        if repo.is_err() {
            error!("please specify the repository path");
            return;
        }

        let logs = repo.unwrap().get_logs(5);
        assert!(logs.is_ok());
        // May have fewer than 5 commits if repo is new
        let log_list = logs.unwrap();
        assert!(log_list.len() <= 5);
    }

    // #[test]
    // fn test_diff() {
    //     let repo = setup();
    //     if repo.is_err() {
    //         error!("please specify the repository path");
    //         return;
    //     }

    //     let diffs = repo.unwrap().get_diff();
    //     assert!(diffs.is_ok());

    //     let diff_content = diffs.unwrap();
    //     assert_ne!(diff_content.len(), 0);
    // }
}
